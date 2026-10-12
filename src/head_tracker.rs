/**
 * # Head Tracker
 * This system its mainly idefied as a tool for allowing "headtracking" for videogames or other systems
 * by introducing the selection between tools like:
 *
 * - Track IR
 * - OpenTrack
 *
 * The main functionality of this system its based on the definition of a "headtracker method" whose for now are 2:
 *
 * 1. TrackIR: The system will search the NPClient64.dll and will try to hook it to get the headtracking data.
 * 2. OpenTrack: The system will listen on a UDP port (4242 by default) for OpenTrack's "UDP over network" output and read the data it sends
 *
 * Once the method its selected we will do a "poll()" allowing the developer to recieve data from the designated method.
*/

use crate::HeadTrackerError;
use crate::opentrack::OpenTrackClient;
#[cfg(windows)]
use crate::trackir::{AxisSelector, TrackIRClient};
use std::fmt;
use std::net::IpAddr;

/// Where the head tracking data comes from.
///
/// It is `#[non_exhaustive]`: more methods may be added later, so a `match` on it needs a `_ =>` arm.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HeadTrackerMethod {
    /// Windows only. Also works for Windows builds running through Wine/Proton.
    #[cfg(windows)]
    TrackIR {
        /// OpenTrack accepts any ID; real TrackIR needs the ID NaturalPoint assigns to your game.
        id: u16,
        /// Which axes to request from the tracking software.
        axes: AxisSelector,
        /// Raw `HWND` of the game window. `None` uses the console window.
        window_handle: Option<isize>,
    },
    OpenTrack {
        /// Must match the port in OpenTrack's "UDP over network" output, usually [`crate::DEFAULT_PORT`].
        port: u16,
        /// Network address to listen on. `0.0.0.0` accepts OpenTrack running on another PC but may show
        /// a Windows firewall prompt; `127.0.0.1` only accepts this PC and avoids the prompt.
        address: IpAddr,
    },
}

/// Head position and rotation in the same units for every method.
/// Rotations are in degrees and positions in centimetres, measured from the centre the tracking software was reset to.
///
/// Signs, from the player's point of view: looking right, looking up, tilting the head right,
/// moving right, moving up and moving closer to the screen are all positive.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct HeadPose {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Running state behind the selected method. Each variant owns whatever its method needs to keep alive.
enum Backend {
    /// Nothing is running, e.g. after `set_origin_method` failed.
    Inactive,
    #[cfg(windows)]
    TrackIR(TrackIRClient),
    OpenTrack(OpenTrackClient),
}

// Written by hand because the TrackIR client holds a loaded DLL with nothing useful to print.
impl fmt::Debug for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backend::Inactive => f.write_str("Inactive"),
            #[cfg(windows)]
            Backend::TrackIR(_) => f.write_str("TrackIR"),
            Backend::OpenTrack(client) => match client.local_addr() {
                Some(addr) => f
                    .debug_struct("OpenTrack")
                    .field("address", &addr.ip())
                    .field("port", &addr.port())
                    .finish(),
                None => f.write_str("OpenTrack"),
            },
        }
    }
}

#[derive(Debug)]
pub struct HeadTracker {
    backend: Backend,
}

impl HeadTracker {
    pub fn new(method: HeadTrackerMethod) -> Result<Self, HeadTrackerError> {
        let mut tracker = HeadTracker {
            backend: Backend::Inactive,
        };
        tracker.set_origin_method(method)?;
        Ok(tracker)
    }

    /**
     * # Set Origin Method
     * Allows you to define a new method for getting the headtracking data, this will allow you to change the method at runtime and start using a new method for getting the data.
     * The previous method is shut down first. If the new one fails to start, the tracker is left inactive and `poll()` returns `None`.
     */
    pub fn set_origin_method(&mut self, method: HeadTrackerMethod) -> Result<(), HeadTrackerError> {
        // Drop the old backend before starting the new one, so TrackIR unregisters before registering again.
        self.backend = Backend::Inactive;
        self.backend = match method {
            #[cfg(windows)]
            HeadTrackerMethod::TrackIR { id, axes, window_handle } => {
                Backend::TrackIR(TrackIRClient::load(id, &axes, window_handle)?)
            }
            HeadTrackerMethod::OpenTrack { port, address } => {
                Backend::OpenTrack(OpenTrackClient::bind(address, port)?)
            }
        };
        Ok(())
    }

    /**
     * # Poll
     * Retrieves the latest headtracking data from the currently selected method.
     * Returns `None` when there is no new frame since the last poll, or when the tracker is inactive.
     */
    pub fn poll(&mut self) -> Option<HeadPose> {
        match &mut self.backend {
            #[cfg(windows)]
            Backend::TrackIR(client) => client.poll().map(|data| data.to_pose()),
            Backend::OpenTrack(client) => client.poll().map(|data| data.to_pose()),
            Backend::Inactive => None,
        }
    }
}
