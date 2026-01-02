//! Core signal-processing and AGP interface for the Rust GMTI platform.
//!
//! The modules mirror the legacy AESADIRP/AIRRADAR pipeline while providing
//! safe abstractions, scoped buffers, and well-defined processing stages.

pub mod agp_interface;
pub mod math;
pub mod prelude;
pub mod processing;
pub mod telemetry;

pub use prelude::{ProcessingStage, StageInput, StageOutput};
