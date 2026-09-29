use super::*;
use crate::config::{self, load_or_default, validate_config, write_default_config, Config};
use crate::daemon::{self, Daemon};
use crate::diagnostics::{print_doctor, run_doctor};
use crate::outputs::VrchatOscOutput;
use crate::providers::{ProviderHub, State};
use crate::util::{config_path, detect_vrchat, find_discord_ipc_sockets, ipc_socket_path};
use anyhow::{bail, Context, Result};
use std::fs;

pub fn run(cli: Cli) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("tokio runtime")?;
    rt.block_on(async move { run_async(cli).await })
}

async fn run_async(cli: Cli) -> Result<()> {
    match cli.cmd {
        Commands::Run { stdout } | Commands::Daemon { stdout } => {
            let cfg = load_or_default()?;
            Daemon::new(cfg, stdout).run().await
        }
        Commands::Status { action } => match action {
            StatusCmd::Set { message } => {
                let msg = message.join(" ");
                let path = ipc_socket_path();
                if path.exists() {
                    let resp = crate::ipc::send_ipc(&path, &format!("status {msg}")).await?;
                    println!("{resp}");
                } else {
                    // Persist override for next daemon start
                    let dir = crate::util::ensure_state_dir()?;
                    fs::write(dir.join("status.override"), &msg)?;
                    println!("wrote status override (daemon not running): {msg}");
                }
                Ok(())
            }
            StatusCmd::Clear => {
                let path = ipc_socket_path();
                if path.exists() {
                    let resp = crate::ipc::send_ipc(&path, "clear").await?;
                    println!("{resp}");
                }
                let dir = crate::util::state_dir()?;
                let f = dir.join("status.override");
                if f.exists() {
                    fs::remove_file(&f).ok();
                }
                println!("cleared");
                Ok(())
            }
            StatusCmd::Show => {
                let cfg = load_or_default()?;
                let text = daemon::preview_once(&cfg).await?;
                println!("{text}");
                Ok(())
            }
        },
        Commands::Preview { profile, watch } => {
            let mut cfg = load_or_default()?;
            if let Some(p) = profile {
                cfg.general.active_profile = p;
            }
            if watch {
                loop {
                    let text = daemon::preview_once(&cfg).await?;
                    println!("{text}");
                    tokio::time::sleep(std::time::Duration::from_secs_f64(
                        cfg.general.tick_secs.max(0.5),
                    ))
                    .await;
                }
            } else {
                let text = daemon::preview_once(&cfg).await?;
                println!("{text}");
                Ok(())
            }
        }
        Commands::Doctor { extra } => {
            let vr = matches!(extra, Some(DoctorCmd::Vrchat));
            let checks = run_doctor(vr)?;
            print_doctor(&checks);
            Ok(())
        }
        Commands::Config { action } => match action {
            ConfigCmd::Init { force } => {
                let path = write_default_config(force)?;
                println!("wrote {}", path.display());
                Ok(())
            }
            ConfigCmd::Path => {
                println!("{}", config_path()?.display());
                Ok(())
            }
            ConfigCmd::Validate { file } => {
                let cfg = validate_config(file.as_deref())?;
                println!(
                    "ok: {} pages, discord.enabled={}, music.mode={:?}",
                    cfg.pages.len(),
                    cfg.discord.enabled,
                    cfg.discord.music.mode
                );
                Ok(())
            }
        },
        Commands::Modules { action } => match action {
            ModulesCmd::List => {
                let cfg = load_or_default()?;
                print_modules(&cfg);
                Ok(())
            }
            ModulesCmd::Enable { module } => set_module(&module, true),
            ModulesCmd::Disable { module } => set_module(&module, false),
        },
        Commands::Enable { module } => set_module(&module, true),
        Commands::Disable { module } => set_module(&module, false),
        Commands::Privacy { action } => match action {
            PrivacyCmd::Audit => privacy_audit().await,
        },
        Commands::Tui => {
            // Minimal first-pass Overview (not full Ratatui app)
            let cfg = load_or_default()?;
            let text = daemon::preview_once(&cfg).await?;
            let vr = detect_vrchat();
            println!("┌─ nixpresence overview (minimal TUI) ─────────────┐");
            println!("│ profile: {:<40} │", cfg.general.active_profile);
            println!(
                "│ vrchat:  {:<40} │",
                if vr.running {
                    format!("yes ({})", vr.via.join(","))
                } else {
                    "no".into()
                }
            );
            println!("│ line:    {:<40} │", truncate_display(&text, 40));
            println!(
                "│ discord: {:<40} │",
                format!("enabled={}", cfg.discord.enabled)
            );
            println!(
                "│ music:   {:<40} │",
                format!("{:?}", cfg.discord.music.mode)
            );
            println!("└──────────────────────────────────────────────────┘");
            println!("(full Ratatui TUI TODO — use `preview --watch` for now)");
            Ok(())
        }
        Commands::Test { target } => match target {
            TestCmd::Osc {
                host,
                port,
                message,
            } => {
                let mut out = VrchatOscOutput::new();
                out.send_force(&host, port, &message, false)?;
                println!("sent OSC /chatbox/input → {host}:{port}: {message}");
                Ok(())
            }
            TestCmd::Discord => {
                let cfg = load_or_default()?;
                let mut d = crate::outputs::DiscordOutput::new();
                match d.connect(&cfg) {
                    Ok(()) => {
                        println!("Discord IPC connect ok");
                        d.clear();
                    }
                    Err(e) => {
                        println!("Discord test: {e}");
                    }
                }
                Ok(())
            }
            TestCmd::Kopuz => {
                let cfg = load_or_default()?;
                let k = crate::providers::kopuz::KopuzProvider::new(&cfg);
                println!(
                    "kopuz socket {}: {}",
                    k.socket_path().display(),
                    if k.socket_present() {
                        "present"
                    } else {
                        "absent (?)"
                    }
                );
                let hub = ProviderHub::new(&cfg);
                let mut st = State::default();
                let _ = hub.mpris.refresh(&cfg, &mut st).await;
                println!(
                    "mpris now: {} — {}",
                    st.player.as_deref().unwrap_or("-"),
                    st.music_display(&cfg)
                );
                Ok(())
            }
        },
        Commands::Ipc { message } => {
            let path = ipc_socket_path();
            let resp = crate::ipc::send_ipc(&path, &message).await?;
            println!("{resp}");
            Ok(())
        }
        Commands::Vrchat { action } => match action {
            VrchatCmd::Detect => {
                let info = detect_vrchat();
                println!(
                    "running={} via={:?} pids={:?}",
                    info.running, info.via, info.pids
                );
                Ok(())
            }
            VrchatCmd::Send { message } => {
                let cfg = load_or_default()?;
                let msg = message.join(" ");
                let mut out = VrchatOscOutput::new();
                out.send_force(&cfg.osc.host, cfg.osc.port, &msg, false)?;
                println!("sent: {msg}");
                Ok(())
            }
        },
        Commands::Discord { action } => match action {
            DiscordCmd::Sockets => {
                for s in find_discord_ipc_sockets() {
                    println!("{}", s.display());
                }
                Ok(())
            }
            DiscordCmd::Music => {
                let cfg = load_or_default()?;
                println!("mode={:?}", cfg.discord.music.mode);
                println!("prefer_players={:?}", cfg.discord.music.prefer_players);
                Ok(())
            }
        },
        Commands::Music { action } => match action {
            MusicCmd::Now => {
                let cfg = load_or_default()?;
                let hub = ProviderHub::new(&cfg);
                let mut st = State::default();
                let _ = hub.mpris.refresh(&cfg, &mut st).await;
                if st.title.is_none() && st.artist.is_none() {
                    println!("(nothing playing via MPRIS)");
                } else {
                    println!(
                        "[{}] {} — {}",
                        st.player.as_deref().unwrap_or("?"),
                        st.artist.as_deref().unwrap_or("?"),
                        st.title.as_deref().unwrap_or("?")
                    );
                }
                Ok(())
            }
        },
    }
}

fn print_modules(cfg: &Config) {
    let m = &cfg.modules;
    let rows = [
        ("system", m.system),
        ("hardware", m.hardware),
        ("nixos", m.nixos),
        ("kernel", m.kernel),
        ("network", m.network),
        ("mpris", m.mpris),
        ("kopuz", m.kopuz),
        ("vrchat", m.vrchat),
        ("custom", m.custom),
    ];
    for (name, on) in rows {
        println!("{name:<12} {}", if on { "enabled" } else { "disabled" });
    }
}

fn set_module(module: &str, enable: bool) -> Result<()> {
    let path = config_path()?;
    if !path.exists() {
        write_default_config(false)?;
    }
    let text = fs::read_to_string(&path)?;
    let key = module.trim().to_lowercase();
    let known = [
        "system", "hardware", "nixos", "kernel", "network", "mpris", "kopuz", "vrchat", "custom",
    ];
    if !known.contains(&key.as_str()) {
        bail!("unknown module '{module}'; known: {}", known.join(", "));
    }
    // Simple line replace under [modules]
    let mut out = String::new();
    let mut in_modules = false;
    let mut done = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_modules = trimmed == "[modules]";
        }
        if in_modules && trimmed.starts_with(&format!("{key} "))
            || in_modules && trimmed.starts_with(&format!("{key}="))
            || in_modules && trimmed.starts_with(&key) && trimmed.contains('=')
        {
            out.push_str(&format!(
                "{key} = {}\n",
                if enable { "true" } else { "false" }
            ));
            done = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !done {
        // append
        if !out.contains("[modules]") {
            out.push_str("\n[modules]\n");
        }
        out.push_str(&format!(
            "{key} = {}\n",
            if enable { "true" } else { "false" }
        ));
    }
    fs::write(&path, out)?;
    println!("{} {}", if enable { "enabled" } else { "disabled" }, key);
    Ok(())
}

async fn privacy_audit() -> Result<()> {
    let cfg = load_or_default()?;
    let text = daemon::preview_once(&cfg).await?;
    println!("composed: {text}");
    println!("privacy.hide_hostname = {}", cfg.privacy.hide_hostname);
    println!("privacy.hide_username = {}", cfg.privacy.hide_username);
    println!("privacy.hide_ip = {}", cfg.privacy.hide_ip);
    println!("privacy.hide_ssid = {}", cfg.privacy.hide_ssid);
    let mut issues = Vec::new();
    if let Ok(u) = std::env::var("USER") {
        if !u.is_empty() && text.contains(&u) {
            issues.push(format!("username '{u}' appears in output"));
        }
    }
    if let Ok(h) = hostname::get() {
        let hs = h.to_string_lossy();
        if cfg.privacy.hide_hostname && text.contains(hs.as_ref()) {
            issues.push(format!("hostname '{hs}' appears in output"));
        }
    }
    let re = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap();
    if re.is_match(&text) {
        issues.push("IPv4-like token appears in output".into());
    }
    if issues.is_empty() {
        println!("audit: clean");
    } else {
        println!("audit: issues");
        for i in issues {
            println!("  - {i}");
        }
    }
    Ok(())
}

fn truncate_display(s: &str, max: usize) -> String {
    let g: Vec<_> = s.chars().collect();
    if g.len() <= max {
        s.to_string()
    } else {
        g.into_iter()
            .take(max.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}

#[allow(dead_code)]
fn _use_config_mod() {
    let _ = config::Config::default();
}
