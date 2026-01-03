use gmticore::agp_interface::DetectionRecord;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisualizationModel {
    pub power_profile: Vec<f32>,
    pub detection_count: usize,
    pub detection_records: Vec<DetectionRecord>,
    pub detection_notes: Vec<String>,
}

#[allow(dead_code)]
impl VisualizationModel {
    pub fn new() -> Self {
        Self {
            power_profile: Vec::new(),
            detection_count: 0,
            detection_records: Vec::new(),
            detection_notes: Vec::new(),
        }
    }
}
