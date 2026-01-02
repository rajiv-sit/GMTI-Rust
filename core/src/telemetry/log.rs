use log::info;

pub struct LogManager;

impl LogManager {
    pub fn new() -> Self {
        Self
    }

    pub fn record(&self, message: &str) {
        info!("{}", message);
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}
