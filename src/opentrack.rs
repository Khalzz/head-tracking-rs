//! OpenTrack backend: listens for OpenTrack's "UDP over network" output.
//! Each frame OpenTrack sends one 48 byte packet with six little-endian `f64` values:
//! x, y, z (centimetres) followed by yaw, pitch, roll (degrees).

use crate::HeadTrackerError;
use crate::head_tracker::HeadPose;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, UdpSocket};

/// Port OpenTrack sends to unless the user changes it in the output settings.
pub const DEFAULT_PORT: u16 = 4242;

const PACKET_SIZE: usize = 48;

/// One frame from OpenTrack. Positions are in centimetres, rotations in degrees.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct OpenTrackData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

impl OpenTrackData {
    fn from_packet(packet: &[u8]) -> Self {
        let value = |i: usize| f64::from_le_bytes(packet[i * 8..i * 8 + 8].try_into().unwrap());
        OpenTrackData {
            x: value(0),
            y: value(1),
            z: value(2),
            yaw: value(3),
            pitch: value(4),
            roll: value(5),
        }
    }

    /// OpenTrack already sends degrees and centimetres. It reports roll, x and z with the opposite sign
    /// to [`HeadPose`], so they are negated.
    pub fn to_pose(&self) -> HeadPose {
        HeadPose {
            yaw: self.yaw as f32,
            pitch: self.pitch as f32,
            roll: -self.roll as f32,
            x: -self.x as f32,
            y: self.y as f32,
            z: -self.z as f32,
        }
    }
}

pub struct OpenTrackClient {
    socket: UdpSocket,
}

impl OpenTrackClient {
    /// Starts listening for OpenTrack packets on `address` and `port`.
    pub fn bind(address: IpAddr, port: u16) -> Result<Self, HeadTrackerError> {
        let socket = UdpSocket::bind((address, port))
            .map_err(|source| HeadTrackerError::Bind { address, port, source })?;
        socket.set_nonblocking(true).map_err(HeadTrackerError::Socket)?;
        Ok(OpenTrackClient { socket })
    }

    /// Address and port the socket is listening on.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.local_addr().ok()
    }

    /// Reads every queued packet and returns the newest one.
    /// Returns `None` when nothing arrived since the last poll.
    pub fn poll(&mut self) -> Option<OpenTrackData> {
        // Larger than a packet so oversized datagrams are detected instead of silently truncated.
        let mut buffer = [0u8; 64];
        let mut latest = None;
        loop {
            match self.socket.recv(&mut buffer) {
                Ok(PACKET_SIZE) => latest = Some(OpenTrackData::from_packet(&buffer[..PACKET_SIZE])),
                // Not an OpenTrack packet, skip it.
                Ok(_) => continue,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_packet() {
        let values = [1.0, 2.0, 3.0, 10.0, 20.0, 30.0];
        let packet: Vec<u8> = values.iter().flat_map(|v: &f64| v.to_le_bytes()).collect();
        assert_eq!(
            OpenTrackData::from_packet(&packet),
            OpenTrackData { x: 1.0, y: 2.0, z: 3.0, yaw: 10.0, pitch: 20.0, roll: 30.0 }
        );
    }

    #[test]
    fn receives_latest_packet() {
        let mut client = OpenTrackClient::bind(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0).unwrap();
        let port = client.socket.local_addr().unwrap().port();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();

        assert_eq!(client.poll(), None);

        for yaw in [1.0, 2.0] {
            let packet: Vec<u8> = [0.0, 0.0, 0.0, yaw, 0.0, 0.0]
                .iter()
                .flat_map(|v: &f64| v.to_le_bytes())
                .collect();
            sender.send_to(&packet, ("127.0.0.1", port)).unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert_eq!(client.poll().map(|d| d.yaw), Some(2.0));
        assert_eq!(client.poll(), None);
    }
}
