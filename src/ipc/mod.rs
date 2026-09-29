//! Unix socket IPC for status set/clear and control.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub enum IpcCommand {
    SetStatus(String),
    ClearStatus,
    SetProfile(String),
    Ping,
    Shutdown,
}

pub struct IpcServer {
    path: PathBuf,
    tx: mpsc::Sender<IpcCommand>,
}

impl IpcServer {
    pub fn new(path: PathBuf, tx: mpsc::Sender<IpcCommand>) -> Self {
        Self { path, tx }
    }

    pub async fn serve(self) -> Result<()> {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let listener = UnixListener::bind(&self.path)
            .with_context(|| format!("bind IPC {}", self.path.display()))?;
        // Restrict to user
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        loop {
            let (stream, _) = listener.accept().await?;
            let tx = self.tx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, tx).await {
                    debug!("IPC client: {e}");
                }
            });
        }
    }
}

async fn handle_client(stream: UnixStream, tx: mpsc::Sender<IpcCommand>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let resp = match parse_command(&line) {
            Ok(cmd) => {
                let _ = tx.send(cmd).await;
                "ok\n"
            }
            Err(e) => {
                warn!("IPC bad cmd: {e}");
                "error\n"
            }
        };
        writer.write_all(resp.as_bytes()).await?;
    }
    Ok(())
}

pub fn parse_command(line: &str) -> Result<IpcCommand> {
    let mut parts = line.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").to_string();
    Ok(match cmd {
        "ping" => IpcCommand::Ping,
        "shutdown" | "quit" => IpcCommand::Shutdown,
        "clear" | "status-clear" => IpcCommand::ClearStatus,
        "status" | "set" => {
            if rest.is_empty() {
                anyhow::bail!("status requires message");
            }
            IpcCommand::SetStatus(rest)
        }
        "profile" => {
            if rest.is_empty() {
                anyhow::bail!("profile requires name");
            }
            IpcCommand::SetProfile(rest)
        }
        other => anyhow::bail!("unknown command: {other}"),
    })
}

pub async fn send_ipc(path: &Path, line: &str) -> Result<String> {
    let stream = UnixStream::connect(path)
        .await
        .with_context(|| format!("connect {}", path.display()))?;
    let (reader, mut writer) = stream.into_split();
    writer.write_all(format!("{line}\n").as_bytes()).await?;
    writer.shutdown().await.ok();
    let mut lines = BufReader::new(reader).lines();
    let resp = lines.next_line().await?.unwrap_or_default();
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ok() {
        match parse_command("status hello world").unwrap() {
            IpcCommand::SetStatus(s) => assert_eq!(s, "hello world"),
            _ => panic!(),
        }
        assert!(matches!(
            parse_command("clear").unwrap(),
            IpcCommand::ClearStatus
        ));
    }
}
