//! TrackIR backend: locates NPClient64.dll through the registry, loads it and reads head tracking frames.
//! Works with the official TrackIR software and with OpenTrack's "freetrack 2.0 Enhanced" output.

use crate::HeadTrackerError;
use crate::head_tracker::HeadPose;
use libloading::Library;
use std::ffi::c_void;
use std::path::PathBuf;
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

/// Frame layout returned by `NP_GetData`. Axis values are in the range of roughly ±16383.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct TrackIRData {
    pub status: u16,
    pub frame_signature: u16,
    pub io_data: u32,
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub raw_x: f32,
    pub raw_y: f32,
    pub raw_z: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    pub smooth_x: f32,
    pub smooth_y: f32,
    pub smooth_z: f32,
}

/// Raw value at the end of every TrackIR axis range.
const NP_AXIS_MAX: f32 = 16383.0;
// Assumed scale, taken from OpenTrack's NPClient: ±NP_AXIS_MAX is ±180 degrees and ±50 cm.
// Verify against the real TrackIR software and adjust here if needed.
const DEGREES_PER_UNIT: f32 = 180.0 / NP_AXIS_MAX;
const CM_PER_UNIT: f32 = 50.0 / NP_AXIS_MAX;

impl TrackIRData {
    /// Converts the raw axis values into degrees and centimetres.
    /// TrackIR reports yaw, pitch, roll, x and z with the opposite sign to [`HeadPose`], so they are negated.
    pub fn to_pose(&self) -> HeadPose {
        HeadPose {
            yaw: -self.yaw * DEGREES_PER_UNIT,
            pitch: -self.pitch * DEGREES_PER_UNIT,
            roll: -self.roll * DEGREES_PER_UNIT,
            x: -self.x * CM_PER_UNIT,
            y: self.y * CM_PER_UNIT,
            z: -self.z * CM_PER_UNIT,
        }
    }
}

type NpRegisterWindowHandle = unsafe extern "system" fn(hwnd: *mut c_void) -> i32;
type NpRegisterProgramProfileId = unsafe extern "system" fn(id: u16) -> i32;
type NpRequestData = unsafe extern "system" fn(flags: u16) -> i32;
type NpGetData = unsafe extern "system" fn(data: *mut TrackIRData) -> i32;
type NpVoidFn = unsafe extern "system" fn() -> i32;

const NP_OK: i32 = 0;

/// Defines if you want to set just all the axes, just some of them or all except some of them.
///
/// Axes that are not requested usually come back as `0` from the official TrackIR software,
/// while OpenTrack may still fill every field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisSelector {
    /// Request every axis in [`Axis::ALL`].
    All,
    /// Request only the listed axes. Duplicates are ignored; an empty list requests nothing
    /// and makes starting the tracker fail.
    Selective(Vec<Axis>),
    /// Request every axis except the listed ones. Listing all six requests nothing
    /// and makes starting the tracker fail.
    AllExcept(Vec<Axis>),
}

/// Axes that can be requested from `NP_RequestData`. Each value is its bit in the request mask.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Rotation from tilting the head towards a shoulder.
    Roll = 0x01,
    /// Rotation from looking up or down.
    Pitch = 0x02,
    /// Rotation from looking left or right.
    Yaw = 0x04,
    /// Translation from moving the head left or right.
    X = 0x10,
    /// Translation from moving the head up or down.
    Y = 0x20,
    /// Translation from moving the head forward or back.
    Z = 0x40,
}

impl Axis {
    /// Every axis: roll, pitch, yaw, x, y, z.
    pub const ALL: [Axis; 6] = [Axis::Roll, Axis::Pitch, Axis::Yaw, Axis::X, Axis::Y, Axis::Z];
}

fn axes_mask(axes: &[Axis]) -> u16 {
    axes.iter().fold(0, |mask, &axis| mask | axis as u16)
}

impl AxisSelector {
    /// Bitmask passed to `NP_RequestData`.
    pub fn mask(&self) -> u16 {
        let all = axes_mask(&Axis::ALL);
        match self {
            AxisSelector::All => all,
            AxisSelector::Selective(axes) => axes_mask(axes),
            AxisSelector::AllExcept(axes) => all & !axes_mask(axes),
        }
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> *mut c_void;
}

fn find_npclient() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\NaturalPoint\NATURALPOINT\NPClient Location")
        .ok()?;
    let dir: String = key.get_value("Path").ok()?;
    Some(PathBuf::from(dir).join("NPClient64.dll"))
}

/// Copies a function pointer out of the library. The caller must keep `library` alive while using it.
unsafe fn symbol<T: Copy>(library: &Library, name: &'static str) -> Result<T, HeadTrackerError> {
    let c_name = format!("{name}\0");
    unsafe { library.get::<T>(c_name.as_bytes()) }
        .map(|s| *s)
        .map_err(|e| HeadTrackerError::MissingFunction { name, source: Box::new(e) })
}

fn check(function: &'static str, code: i32) -> Result<(), HeadTrackerError> {
    if code == NP_OK {
        Ok(())
    } else {
        Err(HeadTrackerError::TrackIRCall { function, code })
    }
}

pub struct TrackIRClient {
    get_data: NpGetData,
    stop_data_transmission: NpVoidFn,
    unregister_window_handle: NpVoidFn,
    last_frame: u16,
    // Must outlive the function pointers above; it is dropped after `Drop::drop` runs.
    _library: Library,
}

impl TrackIRClient {
    /// See [`crate::head_tracker::HeadTrackerMethod::TrackIR`] for what each argument means.
    pub fn load(
        profile_id: u16,
        axes: &AxisSelector,
        window_handle: Option<isize>,
    ) -> Result<Self, HeadTrackerError> {
        let mask = axes.mask();
        if mask == 0 {
            return Err(HeadTrackerError::NoAxesRequested);
        }

        let path = find_npclient().ok_or(HeadTrackerError::NpClientNotFound)?;

        unsafe {
            let library = Library::new(&path).map_err(|e| HeadTrackerError::DllLoad {
                path: path.clone(),
                source: Box::new(e),
            })?;

            let register_window_handle: NpRegisterWindowHandle =
                symbol(&library, "NP_RegisterWindowHandle")?;
            let register_program_profile_id: NpRegisterProgramProfileId =
                symbol(&library, "NP_RegisterProgramProfileID")?;
            let request_data: NpRequestData = symbol(&library, "NP_RequestData")?;
            let start_data_transmission: NpVoidFn = symbol(&library, "NP_StartDataTransmission")?;

            let client = TrackIRClient {
                get_data: symbol(&library, "NP_GetData")?,
                stop_data_transmission: symbol(&library, "NP_StopDataTransmission")?,
                unregister_window_handle: symbol(&library, "NP_UnregisterWindowHandle")?,
                last_frame: 0,
                _library: library,
            };

            let window = match window_handle {
                Some(hwnd) => hwnd as *mut c_void,
                None => {
                    let console = GetConsoleWindow();
                    if console.is_null() {
                        log::warn!(
                            "no window_handle given and there is no console window; \
                             registering without a window, which the real TrackIR software may reject"
                        );
                    } else {
                        log::warn!(
                            "no window_handle given, using the console window; \
                             pass your game window's HWND for the real TrackIR software"
                        );
                    }
                    console
                }
            };
            check("NP_RegisterWindowHandle", register_window_handle(window))?;
            check("NP_RegisterProgramProfileID", register_program_profile_id(profile_id))?;
            check("NP_RequestData", request_data(mask))?;
            check("NP_StartDataTransmission", start_data_transmission())?;

            Ok(client)
        }
    }

    /// Returns `None` when the call fails or there is no new frame since the last poll.
    pub fn poll(&mut self) -> Option<TrackIRData> {
        let mut data = TrackIRData::default();
        if unsafe { (self.get_data)(&mut data) } != NP_OK || data.frame_signature == self.last_frame {
            return None;
        }
        self.last_frame = data.frame_signature;
        Some(data)
    }
}

impl Drop for TrackIRClient {
    fn drop(&mut self) {
        unsafe {
            (self.stop_data_transmission)();
            (self.unregister_window_handle)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_selector_masks() {
        assert_eq!(AxisSelector::All.mask(), 0x77);
        assert_eq!(AxisSelector::Selective(vec![Axis::Roll, Axis::X]).mask(), 0x11);
        assert_eq!(AxisSelector::AllExcept(vec![Axis::X, Axis::Y, Axis::Z]).mask(), 0x07);
        assert_eq!(AxisSelector::Selective(vec![]).mask(), 0);
        assert_eq!(AxisSelector::AllExcept(Axis::ALL.to_vec()).mask(), 0);
    }

    #[test]
    fn converts_to_pose() {
        let data = TrackIRData { yaw: NP_AXIS_MAX, pitch: -NP_AXIS_MAX / 2.0, z: NP_AXIS_MAX, ..Default::default() };
        let pose = data.to_pose();
        assert_eq!(pose.yaw, -180.0);
        assert_eq!(pose.pitch, 90.0);
        assert_eq!(pose.z, -50.0);
    }
}
