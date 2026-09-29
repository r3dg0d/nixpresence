use super::State;
use std::fs;
use std::process::Command;

#[derive(Debug, Default)]
pub struct NixosProvider;

impl NixosProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, state: &mut State) {
        if state.nixos_version.is_some() {
            return;
        }
        // Prefer /etc/os-release BUILD_ID / VERSION_ID
        if let Ok(text) = fs::read_to_string("/etc/os-release") {
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("BUILD_ID=") {
                    state.nixos_version = Some(v.trim().trim_matches('"').to_string());
                    break;
                }
                if let Some(v) = line.strip_prefix("VERSION_ID=") {
                    state.nixos_version = Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
        if state.nixos_version.is_none() {
            if let Ok(out) = Command::new("nixos-version").output() {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !s.is_empty() {
                        state.nixos_version = Some(s);
                    }
                }
            }
        }
        // Enrich os_pretty with short nixos tag if missing nuance
        if state.os_pretty.is_none() {
            state.os_pretty = Some("NixOS".into());
        }
    }
}
