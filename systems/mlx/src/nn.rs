//! Layers shared by the networks, written to match PyTorch's numerics where they differ from MLX's defaults: a Linear
//! runs in its weight's dtype, a LayerNorm in f32, GELU and SiLU in f32 (PyTorch's opmath on half tensors).

use anyhow::Result;
use mlx_rs::{Array, Dtype, fast, ops};

use crate::weights::Weights;

pub fn cast(x: &Array, dtype: Dtype) -> Result<Array> {
    Ok(if x.dtype() == dtype { x.clone() } else { x.as_dtype(dtype)? })
}

pub fn f32(x: &Array) -> Result<Array> {
    cast(x, Dtype::Float32)
}

/// `nn.Linear` at `prefix`: y = x W^T + b, in W's dtype (the input is cast to it, as IDOL's `.type()` calls do).
pub fn linear(w: &Weights, prefix: &str, x: &Array) -> Result<Array> {
    let weight = w.get(&format!("{prefix}.weight"))?;
    let x = cast(x, weight.dtype())?;
    let y = x.matmul(&weight.transpose()?)?;
    Ok(match w.opt(&format!("{prefix}.bias")) {
        Some(b) => ops::add(&y, &cast(b, weight.dtype())?)?,
        None => y,
    })
}

/// LayerNorm over the last axis, computed in f32 and returned in x's dtype (IDOL's LayerNorm32, F.layer_norm).
pub fn layer_norm(x: &Array, weight: Option<&Array>, bias: Option<&Array>, eps: f32) -> Result<Array> {
    let y = fast::layer_norm(f32(x)?, weight, bias, eps)?;
    cast(&y, x.dtype())
}

/// `nn.LayerNorm` at `prefix` with its affine parameters, in f32.
pub fn layer_norm_at(w: &Weights, prefix: &str, x: &Array, eps: f32) -> Result<Array> {
    let weight = w.opt(&format!("{prefix}.weight"));
    let bias = w.opt(&format!("{prefix}.bias"));
    layer_norm(x, weight, bias, eps)
}

/// An elementwise op (an activation) computed in f32 and returned in x's dtype, as PyTorch computes one on a half
/// tensor: `in_f32(&x, mlx_rs::nn::gelu_approximate)` is GELU (tanh), with mlx_rs::nn's gelu and silu alike.
pub fn in_f32(x: &Array, op: impl Fn(Array) -> mlx_rs::error::Result<Array>) -> Result<Array> {
    cast(&op(f32(x)?)?, x.dtype())
}

/// Attention over [B, L, H, D] queries and [B, S, H, D] keys and values -> [B, L, H, D], scaled by 1/sqrt(D) with no
/// mask (PyTorch's F.scaled_dot_product_attention defaults). MLX's fused kernel never forms the L x S matrix.
pub fn attention(q: &Array, k: &Array, v: &Array) -> Result<Array> {
    let d = *q.shape().last().unwrap() as f32;
    let (q, k, v) = (q.swap_axes(1, 2)?, k.swap_axes(1, 2)?, v.swap_axes(1, 2)?);
    let o = fast::scaled_dot_product_attention(&q, &k, &v, 1.0 / d.sqrt(), None, None)?;
    Ok(o.swap_axes(1, 2)?)
}
