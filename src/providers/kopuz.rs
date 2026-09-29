//! Kopuz best-effort integration.
//!
//! Primary path is MPRIS (see `mpris` provider). This module:
//! - detects `$XDG_RUNTIME_DIR/kopuz/kopuzd.sock` for `doctor`
//! - optionally attempts a minimal gRPC GetPlayerState if the socket exists
//!   (currently a documented stub — full tonic codegen omitted to avoid
//!   vendoring EUPL-licensed kopuz proto surface; socket presence is enough
//!   for doctor; MPRIS covers now-playing when Kopuz is running).
use super::State;
use crate::config::Config;
use crate::util::runtime_dir;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug)]
pub struct KopuzProvider {
    socket: PathBuf,
}

impl KopuzProvider {
    pub fn new(cfg: &Config) -> Self {
        let socket = if let Some(p) = &cfg.music.kopuz_socket {
            PathBuf::from(p)
        } else {
            runtime_dir().join("kopuz/kopuzd.sock")
        };
        Self { socket }
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket
    }

    pub fn socket_present(&self) -> bool {
        self.socket.exists()
    }

    /// Best-effort refresh. Returns Some(()) if we attempted / own the music
    /// fields; currently returns None so MPRIS remains primary unless we
    /// later wire tonic.
    pub async fn try_refresh(&self, state: &mut State) -> Option<()> {
        state.kopuz_socket_present = self.socket_present();
        if !state.kopuz_socket_present {
            return None;
        }
        // TODO(grpc): when kopuzd is available, call GetPlayerState over UDS.
        // Schema: kopuz.v1.Kopuz/GetPlayerState → PlayerState.track{title,artist,album}.
        // Intentionally not vendoring the full 1700-line EUPL proto in v0.1;
        // MPRIS covers the same metadata while Kopuz GUI/daemon publishes it.
        debug!(
            path = %self.socket.display(),
            "kopuz socket present; using MPRIS for now-playing (gRPC stub)"
        );
        None
    }
}
