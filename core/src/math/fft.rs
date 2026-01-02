use num_complex::Complex32;
use rustfft::{num_traits::Zero, Fft, FftPlanner};

/// Helper that wraps the `rustfft` planner for reuse.
pub struct FftHelper {
    fft: std::sync::Arc<dyn Fft<f32>>,
    scratch: Vec<Complex32>,
}

impl FftHelper {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(size);
        let scratch = vec![Complex32::zero(); size];
        Self { fft, scratch }
    }

    pub fn forward(&mut self, input: &[f32]) -> Vec<Complex32> {
        let mut buffer: Vec<Complex32> = input
            .iter()
            .map(|&value| Complex32::new(value, 0.0))
            .collect();
        buffer.resize(self.scratch.len(), Complex32::zero());

        self.scratch.copy_from_slice(&buffer);
        self.fft.process(&mut buffer);
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_helper_returns_same_length() {
        let mut helper = FftHelper::new(4);
        let output = helper.forward(&[1.0, 0.0, -1.0, 0.0]);
        assert_eq!(output.len(), 4);
    }
}
