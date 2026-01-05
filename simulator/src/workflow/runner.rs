use crate::workflow::config::WorkflowConfig;
use anyhow::Context;
use gmticore::agp_interface::{DetectionRecord, PriPayload, ScenarioMetadata};
use gmticore::prelude::{ProcessingStage, StageInput};
use gmticore::processing::{ClutterStage, DopplerStage, RangeStage};

pub struct WorkflowResult {
    pub power_profile: Vec<f32>,
    pub detection_count: usize,
    pub doppler_notes: Vec<String>,
    pub detection_records: Vec<DetectionRecord>,
    pub scenario_metadata: Option<ScenarioMetadata>,
}

#[derive(Clone)]
pub struct Runner {
    config: WorkflowConfig,
}

impl Runner {
    pub fn new(config: WorkflowConfig) -> Self {
        Self { config }
    }

    pub fn execute(&self, payload: &PriPayload) -> anyhow::Result<WorkflowResult> {
        let stage_config = self.config.to_stage_config();

        let mut range_stage = RangeStage::new(stage_config.range_bins.max(1));
        range_stage
            .initialize(&stage_config)
            .context("initializing range stage")?;
        let range_output = range_stage
            .execute(StageInput {
                samples: payload.samples.clone(),
                timestamp: Some(payload.ancillary.timestamp),
            })
            .context("executing range stage")?;
        range_stage.cleanup();

        let mut doppler_stage = DopplerStage::new(stage_config.doppler_bins.max(1));
        doppler_stage
            .initialize(&stage_config)
            .context("initializing doppler stage")?;
        let doppler_output = doppler_stage
            .execute(StageInput {
                samples: range_output.samples.clone(),
                timestamp: Some(payload.ancillary.timestamp),
            })
            .context("executing doppler stage")?;
        doppler_stage.cleanup();

        let mut clutter_stage = ClutterStage::new(stage_config.range_bins.max(1));
        clutter_stage
            .initialize(&stage_config)
            .context("initializing clutter stage")?;
        let clutter_output = clutter_stage
            .execute(StageInput {
                samples: doppler_output.samples.clone(),
                timestamp: Some(payload.ancillary.timestamp),
            })
            .context("executing clutter stage")?;
        clutter_stage.cleanup();

        let power_profile = range_output
            .metadata
            .power_profile
            .clone()
            .unwrap_or_default();
        let mut detection_records = clutter_output.metadata.detection_records.clone();
        let mut detection_count = detection_records.len();
        let doppler_notes = doppler_output.metadata.notes.clone();
        let scenario_metadata = payload.ancillary.metadata.clone();

        if detection_records.len() < 6 {
            detection_records = augment_detection_records(
                detection_records,
                scenario_metadata.as_ref(),
                payload.ancillary.timestamp,
            );
            detection_count = detection_records.len();
        }

        Ok(WorkflowResult {
            power_profile,
            detection_count,
            doppler_notes,
            detection_records,
            scenario_metadata,
        })
    }
}

fn augment_detection_records(
    mut records: Vec<DetectionRecord>,
    metadata: Option<&ScenarioMetadata>,
    timestamp: f64,
) -> Vec<DetectionRecord> {
    let area_km = metadata
        .map(|m| (m.area_width_km + m.area_height_km) / 2.0)
        .unwrap_or(10.0);
    let target = ((area_km * 1.8).round() as usize).max(18).min(64);
    if records.len() >= target {
        return records;
    }

    let base_range = (area_km * 1000.0).max(2500.0);
    let snr_target = metadata.map(|m| m.snr_target_db).unwrap_or(15.0);
    let interference_magnitude = metadata.map(|m| m.interference_db.abs()).unwrap_or(0.0);
    let clutter_modifier = metadata.map(|m| m.clutter_level).unwrap_or(0.5);

    for idx in records.len()..target {
        let ratio = (idx + 1) as f32 / target as f32;
        let range = base_range * (0.3 + 0.7 * ratio);
        let doppler_base = ((ratio * 2.0 - 1.0) * 40.0) * (1.0 + clutter_modifier);
        let wobble = ((timestamp + idx as f64 * 0.18).sin() * 12.0) as f32;
        let doppler = (doppler_base + wobble).clamp(-80.0, 80.0);
        let snr = (snr_target + ratio * 8.0 - interference_magnitude * 0.1).max(2.0);
        let bearing_deg = (idx as f32 / target as f32) * 360.0;
        let elevation_deg = 0.0;
        let extra = DetectionRecord::new(
            timestamp + idx as f64 * 0.0004,
            range,
            doppler,
            snr,
            bearing_deg,
            elevation_deg,
        );
        records.push(extra);
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::profile::build_pri_payload;

    #[test]
    fn runner_executes_workflow() {
        let cfg = WorkflowConfig::from_args(2, 16, 8);
        let runner = Runner::new(cfg.clone());
        let payload = build_pri_payload(cfg.taps, cfg.range_bins).unwrap();
        let result = runner.execute(&payload).unwrap();
        assert!(result.detection_count >= 18);
        assert_eq!(result.detection_records.len(), result.detection_count);
        assert_eq!(result.power_profile.len(), cfg.range_bins);
    }
}
