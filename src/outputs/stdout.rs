use tracing::debug;

#[derive(Debug, Default)]
pub struct StdoutOutput {
    pub enabled: bool,
    last: std::sync::Mutex<String>,
}

impl StdoutOutput {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last: std::sync::Mutex::new(String::new()),
        }
    }

    pub fn emit(&self, text: &str) {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if *last == text {
            return;
        }
        *last = text.to_string();
        if self.enabled {
            println!("{text}");
        } else {
            debug!(target: "nixpresence::chatbox", "{text}");
        }
    }
}
