use gmticore::agp_interface::{DetectionRecord, ScenarioMetadata};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisualizationModel {
    pub power_profile: Vec<f32>,
    pub detection_count: usize,
    pub detection_records: Vec<DetectionRecord>,
    pub detection_notes: Vec<String>,
    pub scenario_metadata: Option<ScenarioMetadata>,
}

#[allow(dead_code)]
impl VisualizationModel {
    pub fn new() -> Self {
        Self {
            power_profile: Vec::new(),
            detection_count: 0,
            detection_records: Vec::new(),
            detection_notes: Vec::new(),
            scenario_metadata: None,
        }
    }
}
