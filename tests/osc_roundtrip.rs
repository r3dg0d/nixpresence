//! OSC integration test with ephemeral UDP listener.
use rosc::{OscPacket, OscType};
use std::net::UdpSocket;
use std::time::Duration;

// Re-implement packet build inline to avoid needing library export for tests bin.
// Library unit tests in outputs/vrchat_osc.rs already cover this; this is a crate-level sanity.

fn build_packet(text: &str, notify: bool) -> Vec<u8> {
    let msg = rosc::OscMessage {
        addr: "/chatbox/input".into(),
        args: vec![
            OscType::String(text.into()),
            OscType::Bool(true),
            OscType::Bool(notify),
        ],
    };
    rosc::encoder::encode(&OscPacket::Message(msg)).unwrap()
}

#[test]
fn osc_udp_roundtrip() {
    let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
    listener
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let sender = UdpSocket::bind("0.0.0.0:0").unwrap();
    let bytes = build_packet("nixpresence-itest", false);
    sender.send_to(&bytes, ("127.0.0.1", port)).unwrap();
    let mut buf = [0u8; 2048];
    let (n, _) = listener.recv_from(&mut buf).unwrap();
    let (_, pkt) = rosc::decoder::decode_udp(&buf[..n]).unwrap();
    match pkt {
        OscPacket::Message(m) => {
            assert_eq!(m.addr, "/chatbox/input");
            assert!(matches!(&m.args[0], OscType::String(s) if s == "nixpresence-itest"));
            assert!(matches!(&m.args[1], OscType::Bool(true)));
            assert!(matches!(&m.args[2], OscType::Bool(false)));
        }
        _ => panic!("expected OSC message"),
    }
}
