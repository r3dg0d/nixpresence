//! Generic MPRIS via zbus; prefers players named Kopuz when configured.
use super::State;
use crate::config::Config;
use tracing::debug;
use zbus::proxy;
use zbus::Connection;

#[proxy(
    interface = "org.freedesktop.DBus",
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus"
)]
trait DBus {
    #[zbus(name = "ListNames")]
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait Player {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn metadata(
        &self,
    ) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;
}

#[derive(Debug, Default)]
pub struct MprisProvider;

impl MprisProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn refresh(&self, cfg: &Config, state: &mut State) -> anyhow::Result<()> {
        let conn = match Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                debug!("MPRIS: no session bus ({e})");
                return Ok(());
            }
        };
        let dbus = DBusProxy::new(&conn).await?;
        let names = dbus.list_names().await.unwrap_or_default();
        let mut players: Vec<String> = names
            .into_iter()
            .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
            .collect();

        if players.is_empty() {
            return Ok(());
        }

        // Prefer Kopuz / configured names
        if cfg.music.prefer_kopuz {
            players.sort_by_key(|n| {
                let lower = n.to_lowercase();
                if lower.contains("kopuz") {
                    0
                } else {
                    1
                }
            });
        }

        for name in players {
            let player = PlayerProxy::builder(&conn)
                .destination(name.as_str())?
                .build()
                .await?;
            let status = player.playback_status().await.unwrap_or_default();
            let playing = status.eq_ignore_ascii_case("Playing");
            // Skip stopped unless nothing else
            let meta = match player.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let title = meta_str(&meta, "xesam:title");
            let artist = meta_artists(&meta);
            let album = meta_str(&meta, "xesam:album");
            let art_url = meta_str(&meta, "mpris:artUrl");
            if title.is_none() && artist.is_none() {
                continue;
            }
            // Identity available via MediaPlayer2 proxy if needed later.
            let short_name = name
                .trim_start_matches("org.mpris.MediaPlayer2.")
                .split('.')
                .next()
                .unwrap_or("player")
                .to_string();

            // Prefer playing track
            if playing || state.title.is_none() {
                state.title = title;
                state.artist = artist;
                state.album = album;
                state.art_url = art_url;
                state.player = Some(short_name);
                state.music_playing = playing;
                if playing {
                    break;
                }
            }
        }
        Ok(())
    }
}

fn meta_str(
    meta: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    key: &str,
) -> Option<String> {
    let v = meta.get(key)?;
    if let Ok(s) = <&str>::try_from(&**v) {
        return Some(s.to_string());
    }
    if let Ok(s) = String::try_from(&**v) {
        return Some(s);
    }
    None
}

fn meta_artists(
    meta: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
) -> Option<String> {
    let v = meta.get("xesam:artist")?;
    match &**v {
        zbus::zvariant::Value::Array(arr) => {
            let mut names = Vec::new();
            for item in arr.iter() {
                if let zbus::zvariant::Value::Str(s) = item {
                    names.push(s.as_str().to_string());
                } else if let Ok(s) = <&str>::try_from(item) {
                    names.push(s.to_string());
                }
            }
            if names.is_empty() {
                None
            } else {
                Some(names.join(", "))
            }
        }
        zbus::zvariant::Value::Str(s) => Some(s.as_str().to_string()),
        other => {
            if let Ok(s) = <&str>::try_from(other) {
                Some(s.to_string())
            } else {
                None
            }
        }
    }
}
