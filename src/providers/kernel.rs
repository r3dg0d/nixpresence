use super::State;
use std::fs;

#[derive(Debug, Default)]
pub struct KernelProvider;

impl KernelProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, state: &mut State) {
        if state.kernel.is_some() {
            return;
        }
        if let Ok(v) = fs::read_to_string("/proc/sys/kernel/osrelease") {
            state.kernel = Some(v.trim().to_string());
        } else {
            // fallback uname via std is painful; leave None
        }
    }
}
