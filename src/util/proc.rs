//! Process / VRChat detection helpers.
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct VrchatProcessInfo {
    pub running: bool,
    pub pids: Vec<i32>,
    pub via: Vec<String>,
}

/// Detect VRChat via Steam app 438100, VRChat.exe, and proton/wine parents.
pub fn detect_vrchat() -> VrchatProcessInfo {
    let mut info = VrchatProcessInfo::default();
    let Ok(entries) = fs::read_dir("/proc") else {
        return info;
    };
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let pid: i32 = match name.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let cmdline = read_cmdline(&ent.path());
        let comm = read_comm(&ent.path());
        let exe = read_exe(&ent.path());
        let blob = format!("{cmdline} {comm} {exe}").to_lowercase();

        let mut reasons = Vec::new();
        if blob.contains("vrchat.exe") || blob.contains("vrchat") && blob.contains(".exe") {
            reasons.push("VRChat.exe".into());
        }
        if blob.contains("appid=438100")
            || blob.contains("/438100/")
            || blob.contains("steamappid=438100")
            || cmdline.contains("438100")
        {
            reasons.push("steam-app-438100".into());
        }
        // Proton / wine hierarchy often shows in cmdline
        if (blob.contains("proton") || blob.contains("wine"))
            && (blob.contains("vrchat") || blob.contains("438100"))
        {
            reasons.push("proton-wine".into());
        }
        // Windows-style path under steamapps
        if blob.contains("steamapps/common/vrchat") || blob.contains("steamapps\\common\\vrchat") {
            reasons.push("steamapps-path".into());
        }

        if !reasons.is_empty() {
            info.running = true;
            info.pids.push(pid);
            for r in reasons {
                if !info.via.contains(&r) {
                    info.via.push(r);
                }
            }
        }
    }
    info
}

fn read_cmdline(proc_dir: &Path) -> String {
    fs::read(proc_dir.join("cmdline"))
        .map(|b| String::from_utf8_lossy(&b).replace('\0', " "))
        .unwrap_or_default()
}

fn read_comm(proc_dir: &Path) -> String {
    fs::read_to_string(proc_dir.join("comm"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_exe(proc_dir: &Path) -> String {
    fs::read_link(proc_dir.join("exe"))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Scan `$XDG_RUNTIME_DIR/discord-ipc-*` sockets.
pub fn find_discord_ipc_sockets() -> Vec<PathBuf> {
    let rt = crate::util::runtime_dir();
    let Ok(entries) = fs::read_dir(&rt) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in entries.flatten() {
        let name = ent.file_name();
        let s = name.to_string_lossy();
        if s.starts_with("discord-ipc-") {
            out.push(ent.path());
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_does_not_panic() {
        let _ = detect_vrchat();
    }
}
