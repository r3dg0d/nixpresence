use super::defaults::default_config_toml;
use super::schema::Config;
use crate::util::{config_path, ensure_config_dir};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn load_config(path: &Path) -> Result<Config> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let cfg: Config = toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(cfg)
}

pub fn load_or_default() -> Result<Config> {
    let path = config_path()?;
    if path.exists() {
        load_config(&path)
    } else {
        Ok(Config::default())
    }
}

pub fn write_default_config(force: bool) -> Result<std::path::PathBuf> {
    let dir = ensure_config_dir()?;
    let path = dir.join("config.toml");
    if path.exists() && !force {
        bail!(
            "config already exists at {} (pass --force to overwrite)",
            path.display()
        );
    }
    fs::write(&path, default_config_toml()).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub fn validate_config(path: Option<&Path>) -> Result<Config> {
    let cfg = match path {
        Some(p) => load_config(p)?,
        None => {
            let p = config_path()?;
            if p.exists() {
                load_config(&p)?
            } else {
                Config::default()
            }
        }
    };
    if cfg.general.tick_secs <= 0.0 {
        bail!("general.tick_secs must be > 0");
    }
    if cfg.osc.port == 0 {
        bail!("osc.port must be non-zero");
    }
    if cfg.discord.enabled
        && cfg.discord.application_id.trim().is_empty()
        && std::env::var_os("NIXPRESENCE_DISCORD_APP_ID").is_none()
    {
        bail!("discord.enabled but application_id is empty (set config or NIXPRESENCE_DISCORD_APP_ID)");
    }
    // Refuse known third-party IDs we must never reuse.
    let id = effective_discord_app_id(&cfg);
    if id.as_deref() == Some("1470087339639443658") {
        bail!("refusing Kopuz Discord application id; create your own at discord.com/developers");
    }
    Ok(cfg)
}

pub fn effective_discord_app_id(cfg: &Config) -> Option<String> {
    if let Ok(v) = std::env::var("NIXPRESENCE_DISCORD_APP_ID") {
        if !v.trim().is_empty() {
            return Some(v);
        }
    }
    let id = cfg.discord.application_id.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_parses() {
        let cfg: Config = toml::from_str(default_config_toml()).expect("default toml");
        assert!(cfg.privacy.hide_hostname);
        assert!(matches!(
            cfg.discord.music.mode,
            crate::config::DiscordMusicMode::Coexist
        ));
        assert!(!cfg.discord.enabled);
        assert_eq!(cfg.osc.port, 9000);
        assert!(!cfg.osc.notify);
    }

    #[test]
    fn rejects_kopuz_app_id() {
        let mut cfg = Config::default();
        cfg.discord.enabled = true;
        cfg.discord.application_id = "1470087339639443658".into();
        // validate via toml roundtrip path
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let text = toml::to_string(&cfg).unwrap();
        std::fs::write(tmp.path(), text).unwrap();
        let err = validate_config(Some(tmp.path())).unwrap_err();
        assert!(err.to_string().contains("Kopuz") || err.to_string().contains("1470"));
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;
    use crate::compositor::{render_template, Compositor};
    use crate::providers::State;

    #[test]
    fn privacy_redacts_hostname_in_compose() {
        let mut cfg = Config::default();
        cfg.privacy.hide_hostname = true;
        cfg.pages = vec![crate::config::PageConfig {
            name: "h".into(),
            enabled: true,
            template: "host={hostname} os={os}".into(),
            priority: 1,
            interval_secs: None,
        }];
        cfg.profiles.clear();
        cfg.general.active_profile = "default".into();
        cfg.profiles.insert(
            "default".into(),
            crate::config::ProfileConfig {
                pages: vec!["h".into()],
                rotation: Some(crate::config::RotationConfig {
                    mode: crate::config::RotationMode::Static,
                    ..Default::default()
                }),
            },
        );
        let st = State {
            active_profile: "default".into(),
            hostname: Some("zionsec".into()),
            os_pretty: Some("NixOS".into()),
            ..State::default()
        };
        // hostname token returns empty when hidden
        let rendered = render_template("host={hostname}", &st, &cfg);
        assert_eq!(rendered, "host=");
        let mut c = Compositor::new();
        let out = c.compose(&cfg, &st);
        assert!(!out.contains("zionsec"), "got {out}");
    }
}
