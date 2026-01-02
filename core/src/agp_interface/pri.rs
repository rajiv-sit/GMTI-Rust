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

/// Ancillary metadata accompanying each PRI burst.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriAncillary {
    pub timestamp: f64,
    pub mode: PriType,
    pub pulse_count: usize,
    pub dwell: f32,
    pub range_start: f32,
    pub range_end: f32,
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
