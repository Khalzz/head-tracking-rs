use head_tracking_rs::{HeadTracker, HeadTrackerMethod};

#[test]
fn debug_shows_active_method() {
    let tracker = HeadTracker::new(HeadTrackerMethod::OpenTrack {
        port: 0,
        address: "127.0.0.1".parse().unwrap(),
    })
    .unwrap();
    let text = format!("{tracker:?}");
    assert!(text.starts_with("HeadTracker { backend: OpenTrack { address: 127.0.0.1, port: "), "{text}");
}
