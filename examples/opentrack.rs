use head_tracking_rs::{DEFAULT_PORT, HeadTracker, HeadTrackerMethod};
use std::{thread, time::Duration};

fn main() {
    // Shows the crate's warnings; run with RUST_LOG=debug for more.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut tracker = match HeadTracker::new(HeadTrackerMethod::OpenTrack {
        port: DEFAULT_PORT,
        address: "127.0.0.1".parse().unwrap(),
    }) {
        Ok(tracker) => tracker,
        Err(e) => {
            eprintln!("could not start head tracker: {e}");
            return;
        }
    };

    loop {
        if let Some(pose) = tracker.poll() {
            println!(
                "yaw {:8.1} pitch {:8.1} roll {:8.1} | x {:8.1} y {:8.1} z {:8.1}",
                pose.yaw, pose.pitch, pose.roll, pose.x, pose.y, pose.z
            );
        }
        thread::sleep(Duration::from_millis(16));
    }
}
