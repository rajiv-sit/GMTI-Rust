use ndarray::{Array2, ArrayView2};

pub struct MatrixHelper;

impl MatrixHelper {
    /// Multiply two 2D arrays (all f32 for simplicity).
    pub fn multiply(lhs: ArrayView2<f32>, rhs: ArrayView2<f32>) -> Array2<f32> {
        lhs.dot(&rhs)
    }
}
