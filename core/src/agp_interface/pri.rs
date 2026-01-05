use serde::{Deserialize, Serialize};

/// PRI mode enumeration derived from the legacy `EXT_PriType_Enum`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PriType {
    Standby,
    AdvGmtiScan,
    AdvGmtiStare,
    AdvDmtiStare,
    AdvDmtiScan,
}

/// Describes the operational context for a generated or ingested PRI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMetadata {
    pub name: String,
    pub platform_type: String,
    pub platform_velocity_kmh: f32,
    pub altitude_m: Option<f32>,
    pub area_width_km: f32,
    pub area_height_km: f32,
    pub clutter_level: f32,
    pub snr_target_db: f32,
    pub interference_db: f32,
    pub target_motion: String,
    pub description: Option<String>,
    pub timestamp_start: Option<f64>,
}

/// Ancillary metadata accompanying each PRI burst.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriAncillary {
    pub timestamp: f64,
    pub mode: PriType,
    pub pulse_count: usize,
    pub dwell: f32,
    pub range_start: f32,
    pub range_end: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ScenarioMetadata>,
}

/// Data payload representing a PRI frame consumed by the AGP/processing core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriPayload {
    pub samples: Vec<f32>,
    pub ancillary: PriAncillary,
}

impl PriPayload {
    pub fn new(samples: Vec<f32>, ancillary: PriAncillary) -> Self {
        Self { samples, ancillary }
    }
}
