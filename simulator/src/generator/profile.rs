use anyhow::Context;
use gmticore::agp_interface::{PriAncillary, PriPayload, PriType};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Configuration for generating synthetic PRI data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneratorConfig {
    pub taps: usize,
    pub range_bins: usize,
    pub doppler_bins: usize,
    pub frequency: f32,
    pub noise: f32,
    pub seed: u64,
    pub mode: PriType,
    pub description: Option<String>,
    pub scenario: Option<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            taps: 4,
            range_bins: 2048,
            doppler_bins: 256,
            frequency: 32.0,
            noise: 0.03,
            seed: 0,
            mode: PriType::AdvGmtiScan,
            description: None,
            scenario: None,
        }
    }
}

impl GeneratorConfig {
    fn normalized_taps(&self) -> usize {
        self.taps.max(1)
    }

    fn normalized_range(&self) -> usize {
        self.range_bins.max(1)
    }
}

fn build_sample_vector(config: &GeneratorConfig) -> anyhow::Result<Vec<f32>> {
    let taps = config.normalized_taps();
    let range_bins = config.normalized_range();
    let sample_count = taps
        .checked_mul(range_bins)
        .context("overflow computing sample count for generator")?;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut samples = Vec::with_capacity(sample_count);

    for tap_index in 0..taps {
        let phase_offset = tap_index as f32 * 0.25;
        for range_index in 0..range_bins {
            let base_phase = (range_index as f32 / range_bins as f32) * 2.0 * PI * config.frequency
                + phase_offset;
            let envelope = 0.2 + 0.8 * (1.0 - (range_index as f32 / range_bins as f32));
            let jitter = rng.gen_range(-(config.noise)..config.noise);
            let value = (base_phase).sin() * envelope + jitter;
            samples.push(value);
        }
    }

    Ok(samples)
}

pub fn build_pri_payload_from_config(config: &GeneratorConfig) -> anyhow::Result<PriPayload> {
    let samples = build_sample_vector(config)?;
    let ancillary = PriAncillary {
        timestamp: 0.0,
        mode: config.mode,
        pulse_count: config.normalized_taps(),
        dwell: 45.0,
        range_start: 0.0,
        range_end: 30_000.0,
    };

    Ok(PriPayload::new(samples, ancillary))
}

pub fn build_pri_payload(taps: usize, range_bins: usize) -> anyhow::Result<PriPayload> {
    let config = GeneratorConfig {
        taps,
        range_bins,
        ..Default::default()
    };
    build_pri_payload_from_config(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_builds_expected_sample_count() {
        let payload = build_pri_payload(4, 2048).unwrap();
        assert_eq!(payload.samples.len(), 4 * 2048);
        assert_eq!(payload.ancillary.mode, PriType::AdvGmtiScan);
    }

    #[test]
    fn generator_config_creates_repeated_waveforms() {
        let config = GeneratorConfig {
            taps: 3,
            range_bins: 128,
            doppler_bins: 32,
            frequency: 16.0,
            noise: 0.1,
            seed: 13,
            mode: PriType::AdvDmtiStare,
            description: Some("test".into()),
            scenario: Some("load test".into()),
        };

        let payload = build_pri_payload_from_config(&config).unwrap();
        assert_eq!(payload.samples.len(), 3 * 128);
        assert_eq!(payload.ancillary.mode, PriType::AdvDmtiStare);
    }
}
