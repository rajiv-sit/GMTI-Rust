use anyhow::Context;
use gmticore::prelude::StageConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub taps: usize,
    pub range_bins: usize,
    pub doppler_bins: usize,
}

impl WorkflowConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_ref = path.as_ref();
        let contents = fs::read_to_string(path_ref)
            .with_context(|| format!("reading workflow config {}", path_ref.display()))?;
        let config: WorkflowConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("parsing workflow config {}", path_ref.display()))?;
        Ok(config)
    }

    pub fn from_args(taps: usize, range_bins: usize, doppler_bins: usize) -> Self {
        Self {
            taps,
            range_bins,
            doppler_bins,
        }
    }

    pub fn to_stage_config(&self) -> StageConfig {
        StageConfig {
            taps: self.taps,
            range_bins: self.range_bins,
            doppler_bins: self.doppler_bins,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn config_from_args_produces_stage_config() {
        let cfg = WorkflowConfig::from_args(2, 1024, 128);
        assert_eq!(cfg.to_stage_config().range_bins, 1024);
    }

    #[test]
    fn config_load_reads_yaml() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"taps: 3\nrange_bins: 512\ndoppler_bins: 64\n")
            .unwrap();
        let path = temp.into_temp_path();
        let cfg = WorkflowConfig::load(&path).unwrap();
        assert_eq!(cfg.taps, 3);
    }
}
