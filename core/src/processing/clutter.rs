use crate::agp_interface::DetectionRecord;
use crate::math::stats::StatsHelper;
use crate::prelude::{
    ProcessingStage, StageConfig, StageError, StageInput, StageMetadata, StageOutput, StageResult,
};
use crate::processing::buffer_pool::BufferPool;
use crate::telemetry::log::LogManager;

/// Clutter/detection stage that wraps the final processing step.
pub struct ClutterStage {
    pool: BufferPool,
    config: Option<StageConfig>,
    logger: LogManager,
}

impl ClutterStage {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool: BufferPool::with_capacity(pool_size),
            config: None,
            logger: LogManager::new(),
        }
    }
}

impl ProcessingStage for ClutterStage {
    fn initialize(&mut self, config: &StageConfig) -> StageResult<()> {
        self.config = Some(config.clone());
        Ok(())
    }

    fn execute(&mut self, input: StageInput) -> StageResult<StageOutput> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| StageError::Internal("stage not initialized".into()))?;

        if input.samples.is_empty() {
            return Err(StageError::InvalidInput("no samples to scan".into()));
        }

        let mut buffer = self.pool.checkout(input.samples.len())?;
        buffer.copy_from_slice(&input.samples);

        let threshold = StatsHelper::rms(&buffer) * 1.2;
        let buffer_len = buffer.len() as f32;
        let half_bins = if buffer_len > 0.0 {
            buffer_len / 2.0
        } else {
            0.0
        };

        let mut detection_records = Vec::new();
        let timestamp = input.timestamp.unwrap_or(0.0);
        let range_scale = config.range_bins as f32;

        for (idx, &value) in buffer.iter().enumerate() {
            if value > threshold {
                let range = if buffer_len > 0.0 {
                    range_scale * (idx as f32 / buffer_len)
                } else {
                    0.0
                };
                let doppler = if half_bins > 0.0 {
                    (idx as f32 - half_bins) / half_bins
                } else {
                    0.0
                };
                let snr = value / threshold;
                detection_records.push(DetectionRecord::new(timestamp, range, doppler, snr));
            }
        }

        let detection_count = detection_records.len();
        self.logger
            .record(&format!("ClutterStage detections {}", detection_count));

        let metadata = StageMetadata {
            detection_count: Some(detection_count),
            detection_records,
            notes: vec![format!("threshold {:.3}", threshold)],
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clutter_stage_detects_threshold_exceeding_values() {
        let mut stage = ClutterStage::new(4);
        let config = StageConfig {
            taps: 1,
            range_bins: 4,
            doppler_bins: 4,
        };

        stage.initialize(&config).unwrap();
        let input = StageInput {
            samples: vec![0.1, 20.0, 0.2, 20.0],
            timestamp: Some(0.0),
        };

        let output = stage.execute(input).unwrap();
        assert!(output.metadata.detection_count.unwrap_or(0) >= 2);
        stage.cleanup();
    }
}
