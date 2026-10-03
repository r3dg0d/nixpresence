use super::{format_bytes, State};
use crate::config::Config;
use tracing::{debug, warn};

#[derive(Debug, Default)]
pub struct HardwareProvider {
    enabled: bool,
    gpu_index: u32,
    failed: bool,
    /// Scripted nvidia-smi stdout. `Some` never spawns the binary (tests).
    #[cfg(test)]
    smi_fixture: Option<std::collections::VecDeque<String>>,
}

impl HardwareProvider {
    pub fn new(cfg: &Config) -> Self {
        Self {
            enabled: cfg.hardware.nvml && cfg.modules.hardware,
            gpu_index: cfg.hardware.gpu_index,
            failed: false,
            #[cfg(test)]
            smi_fixture: None,
        }
    }

    pub fn refresh(&mut self, state: &mut State) {
        if !self.enabled || self.failed {
            // NVML is off or already failed. Keep sampling the fallback every
            // refresh: gating on an empty gpu_name froze temp and utilization
            // on the first sample.
            self.refresh_fallback(state);
            return;
        }
        match self.refresh_nvml(state) {
            Ok(()) => {}
            Err(e) => {
                warn!("NVML unavailable ({e}); falling back to nvidia-smi");
                self.failed = true;
                self.refresh_fallback(state);
            }
        }
    }

    fn refresh_fallback(&mut self, state: &mut State) {
        let Some(text) = self.sample_fallback() else {
            return;
        };
        apply_nvidia_smi_csv(state, self.gpu_index, &text);
    }

    #[cfg(test)]
    fn sample_fallback(&mut self) -> Option<String> {
        if let Some(queue) = self.smi_fixture.as_mut() {
            return queue.pop_front();
        }
        query_nvidia_smi()
    }

    #[cfg(not(test))]
    fn sample_fallback(&mut self) -> Option<String> {
        query_nvidia_smi()
    }

    fn refresh_nvml(&self, state: &mut State) -> anyhow::Result<()> {
        // libnvidia-ml lives under /run/opengl-driver/lib on NixOS
        if let Ok(mut path) = std::env::var("LD_LIBRARY_PATH") {
            let extra = "/run/opengl-driver/lib";
            if !path.split(':').any(|p| p == extra) {
                path = format!("{extra}:{path}");
                // Safety: only for this process lookup; nvml-wrapper loads at call time
                std::env::set_var("LD_LIBRARY_PATH", &path);
            }
        } else {
            std::env::set_var("LD_LIBRARY_PATH", "/run/opengl-driver/lib");
        }

        let nvml = nvml_wrapper::Nvml::init()?;
        let count = nvml.device_count()?;
        if count == 0 {
            anyhow::bail!("no NVML devices");
        }
        let idx = self.gpu_index.min(count - 1);
        let dev = nvml.device_by_index(idx)?;
        let name = dev.name().unwrap_or_else(|_| format!("GPU{idx}"));
        // Shorten "NVIDIA GeForce RTX 4090" → "RTX 4090"
        let short = name
            .replace("NVIDIA GeForce ", "")
            .replace("NVIDIA ", "")
            .replace("GeForce ", "");
        state.gpu_name = Some(short);

        if let Ok(temp) =
            dev.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        {
            state.gpu_temp_c = Some(temp as f64);
        }
        if let Ok(util) = dev.utilization_rates() {
            state.gpu_util_pct = Some(util.gpu);
        }
        if let Ok(mem) = dev.memory_info() {
            state.gpu_mem = Some(format!(
                "{}/{}",
                format_bytes(mem.used),
                format_bytes(mem.total)
            ));
        }
        Ok(())
    }
}

fn query_nvidia_smi() -> Option<String> {
    use std::process::Command;
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        debug!("nvidia-smi failed");
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn apply_nvidia_smi_csv(state: &mut State, gpu_index: u32, text: &str) {
    let line = match text.lines().nth(gpu_index as usize) {
        Some(l) => l,
        None => text.lines().next().unwrap_or(""),
    };
    let parts: Vec<_> = line.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return;
    }
    let name = parts[0]
        .replace("NVIDIA GeForce ", "")
        .replace("NVIDIA ", "");
    state.gpu_name = Some(name);
    if parts.len() > 1 {
        if let Ok(t) = parts[1].parse::<f64>() {
            state.gpu_temp_c = Some(t);
        }
    }
    if parts.len() > 2 {
        if let Ok(u) = parts[2].parse::<u32>() {
            state.gpu_util_pct = Some(u);
        }
    }
    if parts.len() > 4 {
        if let (Ok(used), Ok(total)) = (parts[3].parse::<u64>(), parts[4].parse::<u64>()) {
            // nvidia-smi memory is MiB
            state.gpu_mem = Some(format!("{used}M/{total}M"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed_nvml_with_smi(lines: &[&str]) -> HardwareProvider {
        HardwareProvider {
            enabled: true,
            gpu_index: 0,
            failed: true,
            smi_fixture: Some(lines.iter().map(|s| (*s).to_string()).collect()),
        }
    }

    #[test]
    fn fallback_second_refresh_updates_temp_and_util_when_name_is_set() {
        let mut provider = failed_nvml_with_smi(&[
            "NVIDIA GeForce RTX 4090, 55, 10, 1000, 24000",
            "NVIDIA GeForce RTX 4090, 70, 88, 4000, 24000",
        ]);
        let mut state = State {
            gpu_name: Some("RTX 4090".into()),
            gpu_temp_c: Some(41.0),
            gpu_util_pct: Some(3),
            gpu_mem: Some("100M/24000M".into()),
            ..State::default()
        };

        provider.refresh(&mut state);
        assert_eq!(state.gpu_name.as_deref(), Some("RTX 4090"));
        assert_eq!(state.gpu_temp_c, Some(55.0));
        assert_eq!(state.gpu_util_pct, Some(10));
        assert_eq!(state.gpu_mem.as_deref(), Some("1000M/24000M"));

        provider.refresh(&mut state);
        assert_eq!(state.gpu_name.as_deref(), Some("RTX 4090"));
        assert_eq!(state.gpu_temp_c, Some(70.0));
        assert_eq!(state.gpu_util_pct, Some(88));
        assert_eq!(state.gpu_mem.as_deref(), Some("4000M/24000M"));
        assert!(provider.smi_fixture.as_ref().is_some_and(|q| q.is_empty()));
    }
}
