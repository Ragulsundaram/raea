//! raea's MLX facility (docs/mlx.feature): models run on Apple silicon's GPU through MLX (mlx-rs, MIT OR Apache-2.0),
//! beside raea-sys-ml's ONNX Runtime, which stays the path on Windows and Linux and the cross-platform reference.
//!
//! A model's MLX path is written as plain functions over a [`weights::Weights`] map keyed by the checkpoint's own
//! tensor names, so it reads the same safetensors files its PyTorch and ONNX paths do, with the layers in [`nn`]
//! matching PyTorch's numerics. No model uses it yet (IDOL, its first, is held: docs/handoff/idol.md).
//!
//! Precision: MLX runs f32 matmuls (and attention) on the GPU in TF32 unless MLX_ENABLE_TF32=0 is in the environment
//! when the process makes its first matmul, process-wide (like PyTorch's `allow_tf32`; about 1e-3 relative). A model
//! whose parity bar needs exact f32 says so; most run in f16 or bf16 anyway.
//!
//! The crate is empty on every OS but macOS.
#![cfg(target_os = "macos")]

pub mod nn;
pub mod tensor;
pub mod weights;

#[cfg(test)]
mod tests;

/// Points MLX at the metallib to load, ahead of one next to the binary and the path baked in at build time (raea is
/// to fetch it with a model's files on first use, docs/mlx.feature MX4). Call before the first MLX operation.
pub fn use_metallib(path: &std::path::Path) -> anyhow::Result<()> {
    mlx_rs::metal::set_metallib_path(path.to_string_lossy())
        .map_err(|e| anyhow::anyhow!("mlx: can't use metallib {}: {e}", path.display()))
}

/// Whether MLX runs on the GPU here (it falls back to the CPU only where Metal is missing).
pub fn on_gpu() -> bool {
    mlx_rs::Device::default() == mlx_rs::Device::gpu()
}
