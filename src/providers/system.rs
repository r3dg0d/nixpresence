use super::{format_bytes, format_uptime, State};
use crate::util::short_cpu_name;
use std::fs;

#[derive(Debug, Default)]
pub struct SystemProvider {
    cached_cpu: Option<String>,
}

impl SystemProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(&mut self, state: &mut State) {
        if state.os_pretty.is_none() {
            if let Ok(text) = fs::read_to_string("/etc/os-release") {
                for line in text.lines() {
                    if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                        state.os_pretty = Some(unquote(v));
                    } else if let Some(v) = line.strip_prefix("NAME=") {
                        if state.os_pretty.is_none() {
                            state.os_pretty = Some(unquote(v));
                        }
                    } else if let Some(v) = line.strip_prefix("ID=") {
                        state.os_id = Some(unquote(v));
                    } else if let Some(v) = line.strip_prefix("VERSION_ID=") {
                        // used by nixos provider too
                        let _ = v;
                    }
                }
            }
        }

        if self.cached_cpu.is_none() {
            if let Ok(text) = fs::read_to_string("/proc/cpuinfo") {
                for line in text.lines() {
                    if let Some(v) = line.strip_prefix("model name") {
                        let name = v.trim().trim_start_matches(':').trim();
                        self.cached_cpu = Some(name.to_string());
                        break;
                    }
                }
            }
        }
        if let Some(cpu) = &self.cached_cpu {
            state.cpu = Some(cpu.clone());
            state.cpu_short = Some(short_cpu_name(cpu));
        }

        if let Ok(text) = fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut avail_kb = 0u64;
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("MemTotal:") {
                    total_kb = parse_kb(v);
                } else if let Some(v) = line.strip_prefix("MemAvailable:") {
                    avail_kb = parse_kb(v);
                }
            }
            if total_kb > 0 {
                let used = total_kb.saturating_sub(avail_kb) * 1024;
                let total = total_kb * 1024;
                state.ram_used = Some(format_bytes(used));
                state.ram_total = Some(format_bytes(total));
                state.ram = Some(format!("{}/{}", format_bytes(used), format_bytes(total)));
            }
        }

        if let Ok(text) = fs::read_to_string("/proc/uptime") {
            if let Some(first) = text.split_whitespace().next() {
                if let Ok(secs) = first.parse::<f64>() {
                    state.uptime = Some(format_uptime(secs as u64));
                }
            }
        }

        if state.hostname.is_none() {
            state.hostname = hostname::get()
                .ok()
                .map(|h| h.to_string_lossy().into_owned());
        }
    }
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}
