//! Head tracking for games, using TrackIR (NPClient DLL) or OpenTrack (UDP).
//!
//! # Quick start
//!
//! ```no_run
//! use head_tracking_rs::{DEFAULT_PORT, HeadTracker, HeadTrackerMethod};
//!
//! # fn main() -> Result<(), head_tracking_rs::HeadTrackerError> {
//! // Create the tracker once, when the game starts.
//! let mut tracker = HeadTracker::new(HeadTrackerMethod::OpenTrack {
//!     port: DEFAULT_PORT,
//!     address: "127.0.0.1".parse().unwrap(),
//! })?;
//!
//! // Then, every frame:
//! if let Some(pose) = tracker.poll() {
//!     println!("yaw {} pitch {} roll {}", pose.yaw, pose.pitch, pose.roll);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Platform support
//!
//! | Method    | Windows | Linux / macOS                                      |
//! |-----------|---------|----------------------------------------------------|
//! | TrackIR   | yes     | only for Windows builds running through Wine/Proton |
//! | OpenTrack | yes     | yes                                                |
//!
//! TrackIR and its types ([`Axis`], [`AxisSelector`]) only exist when building for Windows.
//!
//! # Logging
//!
//! Warnings (e.g. a missing window handle for TrackIR) go through the [`log`](https://docs.rs/log) crate.
//! Set up any logger, such as `env_logger`, to see them.

mod error;
mod head_tracker;
mod opentrack;
#[cfg(windows)]
mod trackir;

pub use error::HeadTrackerError;
pub use head_tracker::{HeadPose, HeadTracker, HeadTrackerMethod};
pub use opentrack::DEFAULT_PORT;
#[cfg(windows)]
pub use trackir::{Axis, AxisSelector};