use crate::workflow::config::WorkflowConfig;
use anyhow::Context;
use gmticore::agp_interface::{DetectionRecord, PriPayload};
use gmticore::prelude::{ProcessingStage, StageInput};
use gmticore::processing::{ClutterStage, DopplerStage, RangeStage};

pub struct WorkflowResult {
    pub power_profile: Vec<f32>,
    pub detection_count: usize,
    pub doppler_notes: Vec<String>,
    pub detection_records: Vec<DetectionRecord>,
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
        let detection_records = clutter_output.metadata.detection_records.clone();
        let detection_count = detection_records.len();
        let doppler_notes = doppler_output.metadata.notes.clone();

        Ok(WorkflowResult {
            power_profile,
            detection_count,
            doppler_notes,
            detection_records,
        })
    }
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
        assert!(result.detection_count <= cfg.range_bins);
        assert_eq!(result.power_profile.len(), cfg.range_bins);
    }
}
