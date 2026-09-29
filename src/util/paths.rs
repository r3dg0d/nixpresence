use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("dev", "zionsec", "nixpresence")
        .context("could not resolve XDG project directories")
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn state_dir() -> Result<PathBuf> {
    let pd = project_dirs()?;
    Ok(pd
        .state_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| pd.data_local_dir().to_path_buf()))
}

pub fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", nix::unistd::Uid::current())))
}

pub fn ipc_socket_path() -> PathBuf {
    runtime_dir().join("nixpresence.sock")
}

pub fn ensure_config_dir() -> Result<PathBuf> {
    let d = config_dir()?;
    std::fs::create_dir_all(&d).with_context(|| format!("mkdir {}", d.display()))?;
    Ok(d)
}

pub fn ensure_state_dir() -> Result<PathBuf> {
    let d = state_dir()?;
    std::fs::create_dir_all(&d).with_context(|| format!("mkdir {}", d.display()))?;
    Ok(d)
}
