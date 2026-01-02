use serde::{Deserialize, Serialize};

/// Simplified detection record emitted by the processing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionRecord {
    pub timestamp: f64,
    pub range: f32,
    pub doppler: f32,
    pub snr: f32,
}

impl DetectionRecord {
    pub fn new(timestamp: f64, range: f32, doppler: f32, snr: f32) -> Self {
        Self {
            timestamp,
            range,
            doppler,
            snr,
        }
    }
}
