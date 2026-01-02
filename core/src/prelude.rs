use serde::{Deserialize, Serialize};

/// Shared configuration for each processing stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub taps: usize,
    pub range_bins: usize,
    pub doppler_bins: usize,
}

/// Input payload for a processing stage.
#[derive(Debug, Clone)]
pub struct StageInput {
    pub samples: Vec<f32>,
    pub timestamp: Option<f64>,
}

/// Output produced by each stage.
#[derive(Debug, Clone)]
pub struct StageOutput {
    pub samples: Vec<f32>,
    pub metadata: StageMetadata,
}

/// Metadata used for chaining stages and telemetry.
#[derive(Debug, Clone, Default)]
pub struct StageMetadata {
    pub power_profile: Option<Vec<f32>>,
    pub detection_count: Option<usize>,
    pub notes: Vec<String>,
}

/// Common error type for stage execution.
#[derive(thiserror::Error, Debug)]
pub enum StageError {
    #[error("buffer exhaustion: {0}")]
    BufferExhaustion(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("internal failure: {0}")]
    Internal(String),
}

pub type StageResult<T> = Result<T, StageError>;

/// Trait describing object-oriented signal-processing stages.
pub trait ProcessingStage {
    fn initialize(&mut self, config: &StageConfig) -> StageResult<()>;
    fn execute(&mut self, input: StageInput) -> StageResult<StageOutput>;
    fn cleanup(&mut self);
}
