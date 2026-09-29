//! Simple `{token}` and `{?token: ...}` template expander.
use crate::config::Config;
use crate::providers::State;
use crate::util::truncate_graphemes;

pub fn render_template(template: &str, state: &State, cfg: &Config) -> String {
    let mut out = String::with_capacity(template.len());
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            if let Some(end) = find_closing(&chars, i) {
                let inner: String = chars[i + 1..end].iter().collect();
                if let Some(rest) = inner.strip_prefix('?') {
                    // optional section {?key: content} — show content if key non-empty
                    if let Some((key, content)) = rest.split_once(':') {
                        let key = key.trim();
                        let val = lookup(key, state, cfg);
                        if !val.is_empty() {
                            out.push_str(&render_template(content, state, cfg));
                        }
                    }
                } else {
                    let key = inner.trim();
                    out.push_str(&lookup(key, state, cfg));
                }
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn find_closing(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0;
    for (idx, &c) in chars.iter().enumerate().skip(start) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn lookup(key: &str, state: &State, cfg: &Config) -> String {
    let music = state.music_display(cfg);
    match key {
        "os" => state.os_pretty.clone().unwrap_or_default(),
        "os_id" => state.os_id.clone().unwrap_or_default(),
        "kernel" => state.kernel.clone().unwrap_or_default(),
        "cpu" => state.cpu.clone().unwrap_or_default(),
        "cpu_short" => state
            .cpu_short
            .clone()
            .or_else(|| state.cpu.clone())
            .unwrap_or_default(),
        "ram" => state.ram.clone().unwrap_or_default(),
        "ram_used" => state.ram_used.clone().unwrap_or_default(),
        "ram_total" => state.ram_total.clone().unwrap_or_default(),
        "gpu" | "gpu_name" => state.gpu_name.clone().unwrap_or_default(),
        "gpu_temp" => state
            .gpu_temp_c
            .map(|t| format!("{t:.0}"))
            .unwrap_or_default(),
        "gpu_util" => state
            .gpu_util_pct
            .map(|u| format!("{u}"))
            .unwrap_or_default(),
        "gpu_mem" => state.gpu_mem.clone().unwrap_or_default(),
        "music" => music,
        "artist" => state.artist.clone().unwrap_or_default(),
        "title" => state.title.clone().unwrap_or_default(),
        "album" => state.album.clone().unwrap_or_default(),
        "player" => state.player.clone().unwrap_or_default(),
        "custom" => state.custom_text(cfg),
        "hostname" => {
            if cfg.privacy.hide_hostname {
                String::new()
            } else {
                state.hostname.clone().unwrap_or_default()
            }
        }
        "uptime" => state.uptime.clone().unwrap_or_default(),
        "nixos_version" => state.nixos_version.clone().unwrap_or_default(),
        "profile" => state.active_profile.clone(),
        "vrchat" => {
            if state.vrchat_running {
                "VRChat".into()
            } else {
                String::new()
            }
        }
        other => {
            // filters: token|truncate:N
            if let Some((base, filt)) = other.split_once('|') {
                let mut val = lookup(base, state, cfg);
                for f in filt.split('|') {
                    if let Some(n) = f.strip_prefix("truncate:") {
                        if let Ok(max) = n.parse::<usize>() {
                            val = truncate_graphemes(&val, max, &cfg.general.ellipsis);
                        }
                    } else if f == "upper" {
                        val = val.to_uppercase();
                    } else if f == "lower" {
                        val = val.to_lowercase();
                    }
                }
                val
            } else {
                String::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::providers::State;

    #[test]
    fn basic_tokens() {
        let cfg = Config::default();
        let mut st = State {
            os_pretty: Some("NixOS".into()),
            artist: Some("A".into()),
            title: Some("B".into()),
            ..State::default()
        };
        assert_eq!(render_template("{os}", &st, &cfg), "NixOS");
        assert_eq!(
            render_template("x{?music: 🎵 {music}}y", &st, &cfg),
            "x 🎵 A - By"
        );
        st.artist = None;
        st.title = None;
        assert_eq!(render_template("x{?music: 🎵 {music}}y", &st, &cfg), "xy");
    }

    #[test]
    fn truncate_filter() {
        let cfg = Config::default();
        let st = State {
            custom_override: Some("abcdefghijklmnop".into()),
            ..State::default()
        };
        let out = render_template("{custom|truncate:6}", &st, &cfg);
        assert_eq!(crate::util::grapheme_len(&out), 6);
    }
}
