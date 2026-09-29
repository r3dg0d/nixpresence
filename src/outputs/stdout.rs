use tracing::info;

#[derive(Debug, Default)]
pub struct StdoutOutput {
    pub enabled: bool,
}

impl StdoutOutput {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn emit(&self, text: &str) {
        if self.enabled {
            println!("{text}");
        } else {
            info!(target: "nixpresence::preview", "{text}");
        }
    }
}
