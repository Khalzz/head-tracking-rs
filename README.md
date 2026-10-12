# head-tracking-rs

Add head tracking to your Rust game. Players move their head, and you get where they are looking.

It works with:

- **TrackIR**: NaturalPoint's head tracker (Windows).
- **OpenTrack**: free head tracking software that works with a normal webcam, phones and more (Windows, Linux, macOS).

## Install

```toml
[dependencies]
head-tracking-rs = "0.1"
```

## Quick start

Create a tracker once, then ask for the head position every frame:

```rust
use head_tracking_rs::{DEFAULT_PORT, HeadTracker, HeadTrackerMethod};

let mut tracker = HeadTracker::new(HeadTrackerMethod::OpenTrack {
    port: DEFAULT_PORT,
    address: "127.0.0.1".parse().unwrap(),
})?;

// In your game loop:
if let Some(pose) = tracker.poll() {
    println!("looking {} degrees to the side", pose.yaw);
}
```

`poll()` gives you `None` when there's nothing new since the last call, so keep using the last pose you got.

## What you get

Every method gives you the same `HeadPose`:

| Field   | Meaning                        | Positive when you…  |
|---------|--------------------------------|---------------------|
| `yaw`   | turn left/right (degrees)      | look right          |
| `pitch` | look up/down (degrees)         | look up             |
| `roll`  | tilt your head (degrees)       | tilt right          |
| `x`     | move left/right (centimetres)  | move right          |
| `y`     | move up/down (centimetres)     | move up             |
| `z`     | move forward/back (centimetres)| move closer to the screen |

All values start at 0 where the player last centred their tracker.

The values aren't limited or smoothed. Sensitivity, curves and smoothing are set by the player in their tracking software, and any limits your game needs (like how far the camera can turn) are up to you.

## Choosing a method

### OpenTrack (any system)

```rust
HeadTrackerMethod::OpenTrack {
    port: 4242,                          // must match OpenTrack's port
    address: "127.0.0.1".parse().unwrap(),
}
```

In OpenTrack, set **Output** to **UDP over network**, with IP `127.0.0.1` and port `4242`.

Use `0.0.0.0` as the address only if OpenTrack runs on a different PC. Windows may then ask for firewall permission.

### TrackIR (Windows)

```rust
use head_tracking_rs::AxisSelector;

HeadTrackerMethod::TrackIR {
    id: 1001,                 // your game's TrackIR ID
    axes: AxisSelector::All,  // which movements you want
    window_handle: None,      // your game window, see below
}
```

This also works with OpenTrack when its **Output** is set to **freetrack 2.0 Enhanced**.

- **`id`**: the real TrackIR software only works with an ID that NaturalPoint gives your game (apply through their developer program). OpenTrack accepts any number, so you can test with anything.
- **`axes`**: `AxisSelector::All`, only some (`Selective(vec![Axis::Yaw, Axis::Pitch])`), or all except some (`AllExcept(vec![Axis::Z])`).
- **`window_handle`**: your game window's handle (`HWND`). With `None` the console window is used, which is fine for testing but may not work with the real TrackIR software.

TrackIR only exists on Windows. Windows builds of your game running on Linux through Proton or Wine can still use it.

### Switching methods

You can change method while the game runs, for example from a settings menu:

```rust
tracker.set_origin_method(HeadTrackerMethod::OpenTrack { port: 4242, address: "127.0.0.1".parse().unwrap() })?;
```

## When something goes wrong

Starting a tracker returns a `HeadTrackerError` that tells you what happened, so you can show players a helpful message:

```rust
use head_tracking_rs::HeadTrackerError;

match HeadTracker::new(method) {
    Ok(tracker) => { /* use it */ }
    Err(HeadTrackerError::Bind { port, .. }) => println!("port {port} is already in use"),
    Err(e) => println!("head tracking is not available: {e}"),
}
```

Warnings (like a missing window handle) are sent through the [`log`](https://docs.rs/log) crate, so set up a logger such as [`env_logger`](https://docs.rs/env_logger) to see them.

## Examples

```
cargo run --example opentrack
cargo run --example trackir
```

Both print the head position to the terminal.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

TrackIR is a trademark of NaturalPoint, Inc. This project is not affiliated with or endorsed by NaturalPoint.
