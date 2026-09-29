//! VRChat OSC chatbox sender (`/chatbox/input` s b n).
use crate::config::Config;
use crate::providers::State;
use anyhow::{Context, Result};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub const CHATBOX_INPUT: &str = "/chatbox/input";

#[derive(Debug)]
pub struct VrchatOscOutput {
    socket: Option<UdpSocket>,
    last_sent: Option<String>,
    last_at: Option<Instant>,
}

impl Default for VrchatOscOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl VrchatOscOutput {
    pub fn new() -> Self {
        Self {
            socket: None,
            last_sent: None,
            last_at: None,
        }
    }

    fn ensure_socket(&mut self) -> Result<&UdpSocket> {
        if self.socket.is_none() {
            let s = UdpSocket::bind("0.0.0.0:0").context("bind ephemeral UDP for OSC")?;
            s.set_nonblocking(true).ok();
            self.socket = Some(s);
        }
        Ok(self.socket.as_ref().expect("socket just set"))
    }

    /// Build `/chatbox/input` packet: string, send=true, notify=cfg.
    pub fn build_packet(text: &str, notify: bool) -> Result<Vec<u8>> {
        let msg = OscMessage {
            addr: CHATBOX_INPUT.into(),
            args: vec![
                OscType::String(text.to_string()),
                OscType::Bool(true), // send immediately
                OscType::Bool(notify),
            ],
        };
        rosc::encoder::encode(&OscPacket::Message(msg)).context("encode OSC")
    }

    pub fn should_send(&self, cfg: &Config, state: &State, text: &str) -> bool {
        if !cfg.osc.enabled {
            return false;
        }
        if cfg.osc.only_when_vrchat && !cfg.osc.force && !state.vrchat_running {
            return false;
        }
        if let (Some(prev), Some(at)) = (&self.last_sent, self.last_at) {
            let dedup = Duration::from_secs_f64(cfg.osc.dedup_secs.max(0.0));
            if prev == text && at.elapsed() < dedup {
                return false;
            }
        }
        true
    }

    pub fn send(&mut self, cfg: &Config, state: &State, text: &str) -> Result<bool> {
        if !self.should_send(cfg, state, text) {
            debug!("OSC skip (dedup/gate)");
            return Ok(false);
        }
        let bytes = Self::build_packet(text, cfg.osc.notify)?;
        let addr = format!("{}:{}", cfg.osc.host, cfg.osc.port);
        let sock = self.ensure_socket()?;
        sock.send_to(&bytes, &addr)
            .with_context(|| format!("OSC send_to {addr}"))?;
        self.last_sent = Some(text.to_string());
        self.last_at = Some(Instant::now());
        debug!(%addr, len = text.len(), "OSC sent");
        Ok(true)
    }

    /// Force send (for `nixpresence test osc`), ignoring VRChat gate/dedup.
    pub fn send_force(&mut self, host: &str, port: u16, text: &str, notify: bool) -> Result<()> {
        let bytes = Self::build_packet(text, notify)?;
        let addr = format!("{host}:{port}");
        let sock = self.ensure_socket()?;
        sock.send_to(&bytes, &addr)
            .with_context(|| format!("OSC send_to {addr}"))?;
        Ok(())
    }

    pub fn clear(&mut self, cfg: &Config) -> Result<()> {
        let bytes = Self::build_packet("", false)?;
        let addr = format!("{}:{}", cfg.osc.host, cfg.osc.port);
        match self.ensure_socket() {
            Ok(sock) => {
                if let Err(e) = sock.send_to(&bytes, &addr) {
                    warn!("OSC clear failed: {e}");
                }
            }
            Err(e) => warn!("OSC clear: {e}"),
        }
        self.last_sent = Some(String::new());
        self.last_at = Some(Instant::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::time::Duration;

    #[test]
    fn packet_shape() {
        let bytes = VrchatOscOutput::build_packet("hello", false).unwrap();
        let (_, pkt) = rosc::decoder::decode_udp(&bytes).unwrap();
        match pkt {
            OscPacket::Message(m) => {
                assert_eq!(m.addr, CHATBOX_INPUT);
                assert_eq!(m.args.len(), 3);
                assert!(matches!(&m.args[0], OscType::String(s) if s == "hello"));
                assert!(matches!(&m.args[1], OscType::Bool(true)));
                assert!(matches!(&m.args[2], OscType::Bool(false)));
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn integration_udp_listener() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut out = VrchatOscOutput::new();
        out.send_force("127.0.0.1", port, "nixpresence-test", false)
            .unwrap();
        let mut buf = [0u8; 1024];
        let (n, _) = listener.recv_from(&mut buf).expect("recv OSC");
        let (_, pkt) = rosc::decoder::decode_udp(&buf[..n]).unwrap();
        match pkt {
            OscPacket::Message(m) => {
                assert_eq!(m.addr, "/chatbox/input");
                assert!(matches!(&m.args[0], OscType::String(s) if s == "nixpresence-test"));
            }
            _ => panic!("bad packet"),
        }
    }
}
