//! Plain f32 tensors in and out of MLX, the form a backend seam passes (raea-sys-ml's model traits).

use anyhow::{Result, anyhow};
use mlx_rs::{Array, Dtype};

#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl Tensor {
    pub fn new(shape: Vec<usize>, data: Vec<f32>) -> Result<Self> {
        let n: usize = shape.iter().product();
        if n != data.len() {
            return Err(anyhow!("tensor: shape {shape:?} holds {n} values, given {}", data.len()));
        }
        Ok(Self { shape, data })
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        let n = shape.iter().product();
        Self { shape, data: vec![0.0; n] }
    }

    pub fn to_array(&self) -> Array {
        let shape: Vec<i32> = self.shape.iter().map(|&d| d as i32).collect();
        Array::from_slice(&self.data, &shape)
    }

    pub fn from_array(a: &Array) -> Result<Self> {
        // a strided view (a transpose, a slice) is copied to row-major first
        let a = if a.dtype() == Dtype::Float32 { a.clone() } else { a.as_dtype(Dtype::Float32)? }.contiguous()?;
        a.eval()?;
        Ok(Self { shape: a.shape().iter().map(|&d| d as usize).collect(), data: a.as_slice::<f32>().to_vec() })
    }
}
