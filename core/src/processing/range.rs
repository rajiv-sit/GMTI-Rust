use crate::math::stats::StatsHelper;
use crate::prelude::{
    ProcessingStage, StageConfig, StageError, StageInput, StageMetadata, StageOutput, StageResult,
};
use crate::processing::buffer_pool::BufferPool;
use crate::telemetry::log::LogManager;

/// Range-processing stage that mirrors the legacy CPI correction / range compression.
pub struct RangeStage {
    pool: BufferPool,
    config: Option<StageConfig>,
    logger: LogManager,
}

impl RangeStage {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool: BufferPool::with_capacity(pool_size),
            config: None,
            logger: LogManager::new(),
        }
    }
}

impl ProcessingStage for RangeStage {
    fn initialize(&mut self, config: &StageConfig) -> StageResult<()> {
        self.config = Some(config.clone());
        Ok(())
    }

    fn execute(&mut self, input: StageInput) -> StageResult<StageOutput> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| StageError::Internal("stage not initialized".into()))?;

        let expected = config.range_bins * config.taps;
        if input.samples.len() < expected {
            return Err(StageError::InvalidInput(format!(
                "expected at least {} samples",
                expected
            )));
        }

        let mut payload = self.pool.checkout(config.range_bins)?;
        payload.copy_from_slice(&input.samples[..config.range_bins]);

        let power_profile = payload.iter().map(|v| v * v).collect::<Vec<_>>();
        let rms = StatsHelper::rms(&payload);
        self.logger.record(&format!("RangeStage RMS {:.4}", rms));

        let metadata = StageMetadata {
            power_profile: Some(power_profile),
            notes: vec![format!("Range RMS {:.4}", rms)],
            ..Default::default()
        };

        Ok(StageOutput {
            samples: payload,
            metadata,
        })
    }

    fn cleanup(&mut self) {
        self.pool.reset();
        self.config = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_stage_computes_power_profile() {
        let mut stage = RangeStage::new(4);
        let config = StageConfig {
            taps: 1,
            range_bins: 4,
            doppler_bins: 2,
        };

        stage.initialize(&config).unwrap();
        let input = StageInput {
            samples: vec![1.0, 2.0, 3.0, 4.0],
            timestamp: Some(0.0),
        };

        let output = stage.execute(input).unwrap();
        assert_eq!(
            output.metadata.power_profile.unwrap(),
            vec![1.0, 4.0, 9.0, 16.0]
        );
        stage.cleanup();
    }
}
