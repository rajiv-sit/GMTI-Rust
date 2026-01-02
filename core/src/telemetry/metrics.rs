use std::sync::Mutex;

pub struct MetricsRecorder {
    inner: Mutex<Metrics>,
}

struct Metrics {
    processed: usize,
    errors: usize,
}

impl MetricsRecorder {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Metrics {
                processed: 0,
                errors: 0,
            }),
        }
    }

    pub fn record_processed(&self) {
        if let Ok(mut metrics) = self.inner.lock() {
            metrics.processed += 1;
        }
    }

    pub fn record_error(&self) {
        if let Ok(mut metrics) = self.inner.lock() {
            metrics.errors += 1;
        }
    }

    pub fn snapshot(&self) -> (usize, usize) {
        if let Ok(metrics) = self.inner.lock() {
            (metrics.processed, metrics.errors)
        } else {
            (0, 0)
        }
    }
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}
