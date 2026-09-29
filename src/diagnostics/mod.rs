//! `nixpresence doctor` and related checks.
use crate::config::{load_or_default, Config};
use crate::providers::kopuz::KopuzProvider;
use crate::util::{config_path, detect_vrchat, find_discord_ipc_sockets, runtime_dir};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Check {
    pub name: String,
    pub ok: Option<bool>, // None = unknown/?
    pub detail: String,
}

pub fn run_doctor(vrchat_extra: bool) -> Result<Vec<Check>> {
    let mut checks = Vec::new();
    let cfg = load_or_default().unwrap_or_default();

    // Config
    let cpath =
        config_path().unwrap_or_else(|_| PathBuf::from("~/.config/nixpresence/config.toml"));
    checks.push(Check {
        name: "config".into(),
        ok: Some(cpath.exists()),
        detail: if cpath.exists() {
            format!("found {}", cpath.display())
        } else {
            format!(
                "missing {} (using defaults; run `nixpresence config init`)",
                cpath.display()
            )
        },
    });

    // Discord IPC
    let socks = find_discord_ipc_sockets();
    checks.push(Check {
        name: "discord-ipc".into(),
        ok: Some(!socks.is_empty()),
        detail: if socks.is_empty() {
            format!("no discord-ipc-* under {}", runtime_dir().display())
        } else {
            format!(
                "{} socket(s): {}",
                socks.len(),
                socks
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    });

    // Kopuz socket (optional)
    let kopuz = KopuzProvider::new(&cfg);
    let present = kopuz.socket_present();
    checks.push(Check {
        name: "kopuz-grpc".into(),
        ok: if present { Some(true) } else { None }, // ? when absent
        detail: if present {
            format!("socket {}", kopuz.socket_path().display())
        } else {
            format!(
                "? absent {} (MPRIS is primary; gRPC optional)",
                kopuz.socket_path().display()
            )
        },
    });

    // NVML / GPU
    let nvml_lib = PathBuf::from("/run/opengl-driver/lib/libnvidia-ml.so.1");
    let smi = which("nvidia-smi");
    checks.push(Check {
        name: "nvidia".into(),
        ok: Some(nvml_lib.exists() || smi.is_some()),
        detail: format!(
            "libnvidia-ml={} nvidia-smi={}",
            nvml_lib.exists(),
            smi.as_deref().unwrap_or("not-in-PATH")
        ),
    });

    // VRChat
    let vr = detect_vrchat();
    checks.push(Check {
        name: "vrchat".into(),
        ok: if vr.running { Some(true) } else { None },
        detail: if vr.running {
            format!("running via {} (pids {:?})", vr.via.join(","), vr.pids)
        } else {
            "? not running (ok if idle)".into()
        },
    });

    // OSC target
    checks.push(Check {
        name: "osc-target".into(),
        ok: Some(true),
        detail: format!(
            "{}:{} notify={} force={} only_when_vrchat={}",
            cfg.osc.host, cfg.osc.port, cfg.osc.notify, cfg.osc.force, cfg.osc.only_when_vrchat
        ),
    });

    // Discord app id
    let id = crate::config::load::effective_discord_app_id(&cfg);
    let id_ok = match &id {
        Some(i) if i == "1470087339639443658" => Some(false),
        Some(_) => Some(true),
        None => Some(!cfg.discord.enabled),
    };
    checks.push(Check {
        name: "discord-app-id".into(),
        ok: id_ok,
        detail: match id {
            Some(i) if i == "1470087339639443658" => "FORBIDDEN Kopuz id — create your own".into(),
            Some(i) => format!("set ({}…)", i.chars().take(6).collect::<String>()),
            None => {
                if cfg.discord.enabled {
                    "enabled but empty".into()
                } else {
                    "unset (discord disabled — ok)".into()
                }
            }
        },
    });

    // Session bus / MPRIS
    let bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some();
    checks.push(Check {
        name: "mpris-bus".into(),
        ok: Some(bus),
        detail: if bus {
            "DBUS_SESSION_BUS_ADDRESS set".into()
        } else {
            "no session bus env".into()
        },
    });

    // Music mode
    checks.push(Check {
        name: "discord-music-mode".into(),
        ok: Some(true),
        detail: format!("{:?}", cfg.discord.music.mode),
    });

    if vrchat_extra {
        checks.extend(doctor_vrchat_extra(&cfg));
    }

    Ok(checks)
}

fn doctor_vrchat_extra(cfg: &Config) -> Vec<Check> {
    let mut v = Vec::new();
    // Proton path hint
    let home = dirs_home();
    let proton = home.join(".local/share/Steam/compatibilitytools.d");
    v.push(Check {
        name: "steam-compat-tools".into(),
        ok: Some(proton.is_dir()),
        detail: format!("{}", proton.display()),
    });
    let app = home.join(".local/share/Steam/steamapps/common/VRChat");
    v.push(Check {
        name: "vrchat-install".into(),
        ok: if app.exists() { Some(true) } else { None },
        detail: format!("{}", app.display()),
    });
    v.push(Check {
        name: "osc-force".into(),
        ok: Some(true),
        detail: format!("force={} (test with: nixpresence test osc)", cfg.osc.force),
    });
    v
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn which(bin: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        for p in std::env::split_paths(&paths) {
            let cand = p.join(bin);
            if cand.is_file() {
                return Some(cand.display().to_string());
            }
        }
        None
    })
}

pub fn print_doctor(checks: &[Check]) {
    println!("nixpresence doctor");
    println!("{}", "─".repeat(60));
    for c in checks {
        let mark = match c.ok {
            Some(true) => "ok",
            Some(false) => "FAIL",
            None => "?",
        };
        println!("[{mark:>4}] {:<22} {}", c.name, c.detail);
    }
}
