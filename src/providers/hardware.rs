use super::{format_bytes, State};
use crate::config::Config;
use tracing::{debug, warn};

#[derive(Debug, Default)]
pub struct HardwareProvider {
    enabled: bool,
    gpu_index: u32,
    failed: bool,
}

impl HardwareProvider {
    pub fn new(cfg: &Config) -> Self {
        Self {
            enabled: cfg.hardware.nvml && cfg.modules.hardware,
            gpu_index: cfg.hardware.gpu_index,
            failed: false,
        }
    }

    pub fn refresh(&mut self, state: &mut State) {
        if !self.enabled || self.failed {
            // Still try nvidia-smi fallback once names empty
            if state.gpu_name.is_none() {
                self.fallback_nvidia_smi(state);
            }
            return;
        }
        match self.refresh_nvml(state) {
            Ok(()) => {}
            Err(e) => {
                warn!("NVML unavailable ({e}); falling back to nvidia-smi");
                self.failed = true;
                self.fallback_nvidia_smi(state);
            }
        }
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

    fn fallback_nvidia_smi(&self, state: &mut State) {
        use std::process::Command;
        let out = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output();
        let Ok(out) = out else { return };
        if !out.status.success() {
            debug!("nvidia-smi failed");
            return;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let line = match text.lines().nth(self.gpu_index as usize) {
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
}
