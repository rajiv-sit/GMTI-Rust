use crate::math::fft::FftHelper;
use crate::math::stats::StatsHelper;
use crate::prelude::{
    ProcessingStage, StageConfig, StageError, StageInput, StageMetadata, StageOutput, StageResult,
};
use crate::processing::buffer_pool::BufferPool;
use crate::telemetry::log::LogManager;

/// Doppler-stage performing centroid correction and FFT-based power estimation.
pub struct DopplerStage {
    pool: BufferPool,
    config: Option<StageConfig>,
    fft: Option<FftHelper>,
    logger: LogManager,
}

impl DopplerStage {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool: BufferPool::with_capacity(pool_size),
            config: None,
            fft: None,
            logger: LogManager::new(),
        }
    }
}

impl ProcessingStage for DopplerStage {
    fn initialize(&mut self, config: &StageConfig) -> StageResult<()> {
        self.config = Some(config.clone());
        self.fft = Some(FftHelper::new(config.doppler_bins.max(1)));
        Ok(())
    }

    fn execute(&mut self, input: StageInput) -> StageResult<StageOutput> {
        if input.samples.is_empty() {
            return Err(StageError::InvalidInput("no samples provided".into()));
        }

        let fft = self
            .fft
            .as_mut()
            .ok_or_else(|| StageError::Internal("FFT not configured".into()))?;

        let transformed = fft.forward(&input.samples);
        let magnitudes: Vec<f32> = transformed.iter().map(|c| c.norm()).collect();

        let mut buffer = self.pool.checkout(magnitudes.len())?;
        buffer.clone_from_slice(&magnitudes);

        let rms = StatsHelper::rms(&buffer);
        self.logger.record(&format!("Doppler RMS {:.4}", rms));

        let metadata = StageMetadata {
            notes: vec![format!("doppler RMS {:.4}", rms)],
            ..Default::default()
        };

        Ok(StageOutput {
            samples: buffer,
            metadata,
        })
    }

    fn cleanup(&mut self) {
        self.pool.reset();
        self.config = None;
        self.fft = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doppler_stage_returns_magnitude_sequence() {
        let mut stage = DopplerStage::new(8);
        let config = StageConfig {
            taps: 1,
            range_bins: 4,
            doppler_bins: 8,
        };

        stage.initialize(&config).unwrap();
        let input = StageInput {
            samples: vec![1.0, 0.0, 0.0, 0.0],
            timestamp: Some(0.0),
        };

        let output = stage.execute(input).unwrap();
        assert_eq!(output.samples.len(), 8);
        assert!(output.metadata.notes[0].starts_with("doppler RMS"));
        stage.cleanup();
    }
}
