use crate::prelude::StageError;

/// Simple scoped buffer pool that prevents unbounded allocations.
pub struct BufferPool {
    buffers: Vec<Vec<f32>>,
    max_capacity: usize,
}

impl BufferPool {
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            buffers: Vec::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// Allocates a buffer from the pool or creates one if there is room.
    pub fn checkout(&mut self, length: usize) -> Result<Vec<f32>, StageError> {
        if let Some(mut buffer) = self.buffers.pop() {
            buffer.resize(length, 0.0);
            Ok(buffer)
        } else if self.buffers.len() < self.max_capacity {
            Ok(vec![0.0; length])
        } else {
            Err(StageError::BufferExhaustion("pool depleted".to_string()))
        }
    }

    /// Returns a buffer back to the pool for reuse.
    pub fn release(&mut self, mut buffer: Vec<f32>) {
        buffer.clear();
        if self.buffers.len() < self.max_capacity {
            self.buffers.push(buffer);
        }
    }

    pub fn reset(&mut self) {
        self.buffers.clear();
    }
}
