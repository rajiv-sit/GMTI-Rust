use anyhow::Context;
use gmticore::agp_interface::{PriAncillary, PriPayload, PriType, ScenarioMetadata};
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
    pub platform_type: String,
    pub platform_velocity_kmh: f32,
    pub altitude_m: Option<f32>,
    pub area_width_km: f32,
    pub area_height_km: f32,
    pub clutter_level: f32,
    pub snr_target_db: f32,
    pub interference_db: f32,
    pub target_motion: String,
    pub timestamp_start: f64,
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
            platform_type: "Airborne ISR".into(),
            platform_velocity_kmh: 750.0,
            altitude_m: Some(8200.0),
            area_width_km: 10.0,
            area_height_km: 10.0,
            clutter_level: 0.45,
            snr_target_db: 18.0,
            interference_db: -10.0,
            target_motion: "Cruise, gentle zig-zag".into(),
            timestamp_start: 0.0,
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

    let time_offset = config.timestamp_start as f32;
    let motion_signature = config.target_motion.bytes().fold(0u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u64)
    });
    let motion_bias = (motion_signature as f32 % 360.0).to_radians();
    let snr_linear = 10f32.powf(config.snr_target_db / 20.0);
    let interference_amplitude = 10f32.powf(config.interference_db / 20.0);
    let speed_factor = (config.platform_velocity_kmh / 500.0).min(3.0);

    for tap_index in 0..taps {
        let phase_offset = tap_index as f32 * 0.25;
        for range_index in 0..range_bins {
            let normalized_range = range_index as f32 / range_bins as f32;
            let base_phase = (normalized_range + time_offset * 0.0001 + phase_offset * 0.01)
                * 2.0
                * PI
                * config.frequency;
            let envelope = 0.2 + 0.8 * (1.0 - normalized_range);
            let jitter = rng.gen_range(-(config.noise)..config.noise);
            let time_wave =
                (time_offset * 0.02 * speed_factor + normalized_range * 2.0 + motion_bias).sin();
            let motion_wobble = ((normalized_range * 8.0) + motion_bias + time_offset * 0.05).sin();
            let clutter_jitter = config.clutter_level * rng.gen_range(-1.0..1.0);
            let interference =
                interference_amplitude * (normalized_range * 4.0 + time_offset * 0.08).cos();
            let snr_component = snr_linear * motion_wobble * (1.0 - normalized_range * 0.6);
            let value = (base_phase + phase_offset + time_wave).sin()
                * envelope
                * (1.0 + 0.3 * motion_wobble * speed_factor)
                + clutter_jitter
                + interference
                + snr_component
                + jitter;
            samples.push(value);
        }
    }

    Ok(samples)
}

pub fn build_pri_payload_from_config(config: &GeneratorConfig) -> anyhow::Result<PriPayload> {
    let samples = build_sample_vector(config)?;
    let scenario_name = config
        .scenario
        .clone()
        .unwrap_or_else(|| "generated-burst".into());
    let ancillary = PriAncillary {
        timestamp: config.timestamp_start,
        mode: config.mode,
        pulse_count: config.normalized_taps(),
        dwell: 45.0,
        range_start: 0.0,
        range_end: 30_000.0,
        metadata: Some(ScenarioMetadata {
            name: scenario_name,
            platform_type: config.platform_type.clone(),
            platform_velocity_kmh: config.platform_velocity_kmh,
            altitude_m: config.altitude_m,
            area_width_km: config.area_width_km,
            area_height_km: config.area_height_km,
            clutter_level: config.clutter_level,
            snr_target_db: config.snr_target_db,
            interference_db: config.interference_db,
            target_motion: config.target_motion.clone(),
            description: config.description.clone(),
            timestamp_start: Some(config.timestamp_start),
        }),
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
        let mut config = GeneratorConfig {
            taps: 3,
            range_bins: 128,
            doppler_bins: 32,
            ..Default::default()
        };
        config.frequency = 16.0;
        config.noise = 0.1;
        config.seed = 13;
        config.mode = PriType::AdvDmtiStare;
        config.description = Some("test".into());
        config.scenario = Some("load test".into());

        let payload = build_pri_payload_from_config(&config).unwrap();
        assert_eq!(payload.samples.len(), 3 * 128);
        assert_eq!(payload.ancillary.mode, PriType::AdvDmtiStare);
    }
}
