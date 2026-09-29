//! Focused window detection (Hyprland / X11) for Discord small_image.
use super::State;
use serde_json::Value;
use std::process::Command;
use tracing::debug;

#[derive(Debug, Default)]
pub struct FocusedAppProvider;

impl FocusedAppProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, state: &mut State) {
        state.focused_app = None;
        state.focused_title = None;

        if let Some((class, title)) = detect_hyprland() {
            state.focused_app = Some(class);
            state.focused_title = title;
            return;
        }
        if let Some((class, title)) = detect_x11() {
            state.focused_app = Some(class);
            state.focused_title = title;
        }
    }
}

/// Resolve a tool that systemd units often miss (minimal PATH).
fn resolve_bin(name: &str) -> Option<std::path::PathBuf> {
    // Absolute / relative path as given
    let as_path = std::path::Path::new(name);
    if as_path.is_absolute() && as_path.is_file() {
        return Some(as_path.to_path_buf());
    }
    // PATH lookup
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|s| !s.is_empty()) {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // NixOS / common fallbacks (user systemd often strips PATH)
    for dir in ["/run/current-system/sw/bin", "/usr/bin", "/bin"] {
        let candidate = std::path::Path::new(dir).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let local = std::path::Path::new(&home).join(".local/bin").join(name);
        if local.is_file() {
            return Some(local);
        }
        // /etc/profiles/per-user/<user>/bin
        if let Some(user) = std::path::Path::new(&home).file_name() {
            let prof = std::path::Path::new("/etc/profiles/per-user")
                .join(user)
                .join("bin")
                .join(name);
            if prof.is_file() {
                return Some(prof);
            }
        }
    }
    None
}

/// Run a binary with a clean dynamic linker env so NixOS system tools
/// (hyprctl) are not broken by cargo/dev LD_LIBRARY_PATH pollution.
fn run_clean(bin: &str, args: &[&str]) -> Option<String> {
    let Some(path) = resolve_bin(bin) else {
        debug!(
            bin,
            "focused-app binary not found on PATH or NixOS fallbacks"
        );
        return None;
    };
    let mut cmd = Command::new(&path);
    cmd.args(args);
    cmd.env_remove("LD_LIBRARY_PATH");
    cmd.env_remove("LD_PRELOAD");
    // Ensure hyprland socket vars survive even if somehow cleared
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        Ok(out) => {
            debug!(
                bin,
                status = ?out.status,
                stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                "focused-app probe failed"
            );
            None
        }
        Err(e) => {
            debug!(bin, error = %e, "focused-app binary missing");
            None
        }
    }
}

fn detect_hyprland() -> Option<(String, Option<String>)> {
    // Prefer hyprctl when HYPRLAND_INSTANCE_SIGNATURE is set (or socket dir exists).
    let has_sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
    let rt = crate::util::runtime_dir();
    let has_sock = rt.join("hypr").is_dir();
    if !has_sig && !has_sock {
        return None;
    }
    let json = run_clean("hyprctl", &["activewindow", "-j"])?;
    if json == "{}" || json == "null" {
        return None;
    }
    let v: Value = serde_json::from_str(&json).ok()?;
    let class = v
        .get("class")
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let title = v
        .get("title")
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some((class, title))
}

fn detect_x11() -> Option<(String, Option<String>)> {
    // xprop -id $(xdotool getactivewindow) WM_CLASS WM_NAME
    let id = run_clean("xdotool", &["getactivewindow"])?;
    let out = run_clean("xprop", &["-id", &id, "WM_CLASS", "WM_NAME"])?;
    let mut class = None;
    let mut title = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("WM_CLASS(STRING) = ") {
            // "instance", "Class"
            let parts: Vec<&str> = rest.split(',').collect();
            if let Some(last) = parts.last() {
                let c = last.trim().trim_matches('"');
                if !c.is_empty() {
                    class = Some(c.to_string());
                }
            }
        } else if let Some(rest) = line.strip_prefix("WM_NAME(STRING) = ") {
            let t = rest.trim().trim_matches('"');
            if !t.is_empty() {
                title = Some(t.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("WM_NAME(UTF8_STRING) = ") {
            let t = rest.trim().trim_matches('"');
            if !t.is_empty() {
                title = Some(t.to_string());
            }
        }
    }
    class.map(|c| (c, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_does_not_panic() {
        let p = FocusedAppProvider::new();
        let mut state = State::default();
        p.refresh(&mut state);
    }
}
