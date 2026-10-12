use std::error::Error;
use std::fmt;
use std::net::IpAddr;
use std::path::PathBuf;

/// Everything that can go wrong while starting a head tracking method.
///
/// It is `#[non_exhaustive]`, so match it with a `_ =>` arm; new cases can then be added without breaking your code.
#[derive(Debug)]
#[non_exhaustive]
pub enum HeadTrackerError {
    /// The registry has no NPClient location, so neither TrackIR nor OpenTrack's freetrack output has run on this PC.
    NpClientNotFound,
    /// NPClient64.dll was found but could not be loaded.
    DllLoad {
        path: PathBuf,
        source: Box<dyn Error + Send + Sync>,
    },
    /// The DLL does not export one of the `NP_` functions this crate needs.
    MissingFunction {
        name: &'static str,
        source: Box<dyn Error + Send + Sync>,
    },
    /// A TrackIR call returned an error code, e.g. 7 (already registered) or 8 (unknown profile ID).
    TrackIRCall { function: &'static str, code: i32 },
    /// The `AxisSelector` did not select any axis.
    NoAxesRequested,
    /// The UDP port for OpenTrack could not be opened, usually because another program is using it.
    Bind {
        address: IpAddr,
        port: u16,
        source: std::io::Error,
    },
    /// The UDP socket was opened but could not be configured.
    Socket(std::io::Error),
}

/// Name of a TrackIR error code, as defined by NPClient.
fn np_error_name(code: i32) -> &'static str {
    match code {
        1 => "NP_ERR_DEVICE_NOT_PRESENT",
        2 => "NP_ERR_UNSUPPORTED_OS",
        3 => "NP_ERR_INVALID_ARG",
        4 => "NP_ERR_DLL_NOT_FOUND",
        5 => "NP_ERR_NO_DATA",
        6 => "NP_ERR_INTERNAL_DATA",
        7 => "NP_ERR_ALREADY_REGISTERED",
        8 => "NP_ERR_UNKNOWN_ID (profile ID not recognised by the TrackIR software)",
        9 => "NP_ERR_READ_ONLY",
        _ => "unknown error",
    }
}

impl fmt::Display for HeadTrackerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeadTrackerError::NpClientNotFound => {
                write!(f, "NPClient location not found in registry (is TrackIR or OpenTrack installed?)")
            }
            HeadTrackerError::DllLoad { path, source } => write!(f, "failed to load {}: {source}", path.display()),
            HeadTrackerError::MissingFunction { name, source } => write!(f, "{name} not found: {source}"),
            HeadTrackerError::TrackIRCall { function, code } => {
                write!(f, "{function} failed: {} (code {code})", np_error_name(*code))
            }
            HeadTrackerError::NoAxesRequested => write!(f, "no axes requested; use AxisSelector::All or list at least one axis"),
            HeadTrackerError::Bind { address, port, source } => {
                write!(f, "could not listen on {address}:{port} (is another program using it?): {source}")
            }
            HeadTrackerError::Socket(source) => write!(f, "could not configure UDP socket: {source}"),
        }
    }
}

impl Error for HeadTrackerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HeadTrackerError::DllLoad { source, .. } | HeadTrackerError::MissingFunction { source, .. } => {
                Some(source.as_ref())
            }
            HeadTrackerError::Bind { source, .. } | HeadTrackerError::Socket(source) => Some(source),
            HeadTrackerError::NpClientNotFound
            | HeadTrackerError::TrackIRCall { .. }
            | HeadTrackerError::NoAxesRequested => None,
        }
    }
}
