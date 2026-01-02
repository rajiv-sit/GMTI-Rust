use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisualizationModel {
    pub power_profile: Vec<f32>,
    pub detection_count: usize,
}

#[allow(dead_code)]
impl VisualizationModel {
    pub fn new() -> Self {
        Self {
            power_profile: Vec::new(),
            detection_count: 0,
        }
    }
}
