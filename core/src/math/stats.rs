pub struct StatsHelper;

impl StatsHelper {
    pub fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|&v| v * v).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_zero_sequence_yields_zero() {
        assert_eq!(StatsHelper::rms(&[]), 0.0);
        assert_eq!(StatsHelper::rms(&[0.0, 0.0]), 0.0);
    }

    #[test]
    fn rms_handles_single_value() {
        assert_eq!(StatsHelper::rms(&[4.0]), 4.0);
    }
}
