//! Main tick loop: providers → compositor → outputs.
use crate::compositor::Compositor;
use crate::config::Config;
use crate::ipc::{IpcCommand, IpcServer};
use crate::outputs::{DiscordOutput, StdoutOutput, VrchatOscOutput};
use crate::providers::{ProviderHub, State};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

pub struct Daemon {
    pub cfg: Config,
    pub state: Arc<RwLock<State>>,
    pub stdout: bool,
}

impl Daemon {
    pub fn new(cfg: Config, stdout: bool) -> Self {
        let state = State {
            active_profile: cfg.general.active_profile.clone(),
            ..Default::default()
        };
        Self {
            cfg,
            state: Arc::new(RwLock::new(state)),
            stdout,
        }
    }

    pub async fn run(self) -> Result<()> {
        let mut hub = ProviderHub::new(&self.cfg);
        let mut compositor = Compositor::new();
        let mut osc = VrchatOscOutput::new();
        let mut discord = DiscordOutput::new();
        let stdout = StdoutOutput::new(self.stdout);

        let (tx, mut rx) = mpsc::channel::<IpcCommand>(32);
        let ipc_state = self.state.clone();
        let ipc_path = self
            .cfg
            .ipc
            .socket
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(crate::util::ipc_socket_path);

        if self.cfg.ipc.enabled {
            let server = IpcServer::new(ipc_path.clone(), tx);
            tokio::spawn(async move {
                if let Err(e) = server.serve().await {
                    warn!("IPC server ended: {e}");
                }
            });
            info!(path = %ipc_path.display(), "IPC listening");
        }

        // Initial Discord connect (best-effort)
        if self.cfg.discord.enabled {
            if let Err(e) = discord.connect(&self.cfg) {
                warn!("Discord not ready yet: {e}");
            }
        }

        let tick = Duration::from_secs_f64(self.cfg.general.tick_secs.max(0.2));
        info!(
            tick_ms = tick.as_millis(),
            profile = %self.cfg.general.active_profile,
            "daemon started"
        );

        let mut interval = tokio::time::interval(tick);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    {
                        let mut st = self.state.write().await;
                        hub.refresh(&self.cfg, &mut st).await;
                        let text = compositor.compose(&self.cfg, &st);
                        stdout.emit(&text);
                        if let Err(e) = osc.send(&self.cfg, &st, &text) {
                            warn!("OSC: {e}");
                        }
                        if let Err(e) = discord.publish(&self.cfg, &st, &text) {
                            warn!("Discord: {e}");
                        }
                    }
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(IpcCommand::SetStatus(msg)) => {
                            let mut st = self.state.write().await;
                            st.custom_override = Some(msg);
                        }
                        Some(IpcCommand::ClearStatus) => {
                            let mut st = self.state.write().await;
                            st.custom_override = None;
                        }
                        Some(IpcCommand::SetProfile(p)) => {
                            let mut st = self.state.write().await;
                            st.active_profile = p;
                        }
                        Some(IpcCommand::Shutdown) => {
                            info!("IPC shutdown");
                            let _ = osc.clear(&self.cfg);
                            discord.clear();
                            break;
                        }
                        Some(IpcCommand::Ping) => {}
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("SIGINT");
                    let _ = osc.clear(&self.cfg);
                    discord.clear();
                    break;
                }
            }
        }
        let _ = ipc_state;
        Ok(())
    }
}

/// One-shot preview without daemon loop.
pub async fn preview_once(cfg: &Config) -> Result<String> {
    let mut hub = ProviderHub::new(cfg);
    let mut state = State {
        active_profile: cfg.general.active_profile.clone(),
        ..Default::default()
    };
    hub.refresh(cfg, &mut state).await;
    let mut compositor = Compositor::new();
    Ok(compositor.compose(cfg, &state))
}
