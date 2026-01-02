use std::f32::consts::PI;

/// Generates a simple sine waveform for dummy CPI data.
#[allow(dead_code)]
pub fn sine_wave(length: usize, frequency: f32) -> Vec<f32> {
    (0..length)
        .map(|i| ((i as f32 * frequency) / length as f32 * 2.0 * PI).sin())
        .collect()
}
