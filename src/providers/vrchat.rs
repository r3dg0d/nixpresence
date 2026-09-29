use super::State;
use crate::util::detect_vrchat;

#[derive(Debug, Default)]
pub struct VrchatProvider;

impl VrchatProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, state: &mut State) {
        let info = detect_vrchat();
        state.vrchat_running = info.running;
        state.vrchat_via = info.via;
    }
}
