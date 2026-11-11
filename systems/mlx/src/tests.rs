//! Each layer on the GPU against the same maths written out on the CPU in plain Rust (docs/mlx.feature MX2).

use mlx_rs::{Array, Dtype};

use crate::nn;
use crate::tensor::Tensor;
use crate::weights::Weights;

/// The bound on an f32 matmul's error for values near 1. MLX runs f32 matmuls in TF32 on the GPU by default
/// (MLX_ENABLE_TF32=1): about 1e-3 off here (11-10: 1.2e-3 on the linear test). With MLX_ENABLE_TF32=0 all of these
/// pass at 1e-5. A wrong transpose or axis is off by order 1, so the bound still catches the bugs it's for.
const MATMUL_F32: f32 = 3e-3;

/// Deterministic values in -1..1 (no rand dependency).
fn values(n: usize, seed: u64) -> Vec<f32> {
    let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
        })
        .collect()
}

fn read(a: &Array) -> Vec<f32> {
    Tensor::from_array(a).unwrap().data
}

fn max_err(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

fn linear_weights(o: usize, i: usize, dtype: Dtype) -> (Weights, Vec<f32>, Vec<f32>) {
    let (w, b) = (values(o * i, 1), values(o, 2));
    let mut weights: Weights =
        [("fc.weight", Array::from_slice(&w, &[o as i32, i as i32])), ("fc.bias", Array::from_slice(&b, &[o as i32]))]
            .into_iter()
            .collect();
    weights.cast_where(dtype, |_| true).unwrap();
    (weights, w, b)
}

#[test]
fn runs_on_the_gpu() {
    assert!(crate::on_gpu(), "MLX's default device on this Mac should be the GPU");
}

#[test]
fn linear_matches_the_cpu() {
    let (n, i, o) = (6, 8, 5);
    let (weights, w, b) = linear_weights(o, i, Dtype::Float32);
    let x = values(n * i, 3);
    let y = nn::linear(&weights, "fc", &Array::from_slice(&x, &[2, 3, i as i32])).unwrap();
    assert_eq!(y.shape(), &[2, 3, o as i32]);
    let want: Vec<f32> = (0..n)
        .flat_map(|r| (0..o).map(move |c| (r, c)))
        .map(|(r, c)| b[c] + (0..i).map(|k| x[r * i + k] * w[c * i + k]).sum::<f32>())
        .collect();
    let got = read(&y);
    let err = max_err(&got, &want);
    assert!(err < MATMUL_F32, "max error {err}: got {:?}, want {:?}", &got[..5], &want[..5]);
}

#[test]
fn a_half_linear_runs_in_half_and_stays_close() {
    let (weights, _, _) = linear_weights(5, 8, Dtype::Float16);
    let (full, _, _) = linear_weights(5, 8, Dtype::Float32);
    let x = Array::from_slice(&values(24, 3), &[3, 8]);
    let half = nn::linear(&weights, "fc", &x).unwrap();
    assert_eq!(half.dtype(), Dtype::Float16, "an f16 Linear casts its input and answers in f16, as PyTorch's does");
    assert!(max_err(&read(&half), &read(&nn::linear(&full, "fc", &x).unwrap())) < 1e-2);
}

#[test]
fn layer_norm_matches_the_cpu_and_keeps_the_input_dtype() {
    let (rows, d) = (4, 16);
    let x = values(rows * d, 4);
    let y = nn::layer_norm(&Array::from_slice(&x, &[rows as i32, d as i32]), None, None, 1e-6).unwrap();
    let want: Vec<f32> = x
        .chunks(d)
        .flat_map(|r| {
            let mean = r.iter().sum::<f32>() / d as f32;
            let var = r.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / d as f32;
            r.iter().map(move |v| (v - mean) / (var + 1e-6).sqrt()).collect::<Vec<_>>()
        })
        .collect();
    assert!(max_err(&read(&y), &want) < 1e-5);
    let half = Array::from_slice(&x, &[rows as i32, d as i32]).as_dtype(Dtype::Float16).unwrap();
    assert_eq!(nn::layer_norm(&half, None, None, 1e-6).unwrap().dtype(), Dtype::Float16);
}

#[test]
fn attention_matches_the_cpu() {
    let (l, s, h, d) = (5, 7, 2, 4);
    let (q, k, v) = (values(l * h * d, 5), values(s * h * d, 6), values(s * h * d, 7));
    let arr = |x: &[f32], n: usize| Array::from_slice(x, &[1, n as i32, h as i32, d as i32]);
    let o = nn::attention(&arr(&q, l), &arr(&k, s), &arr(&v, s)).unwrap();
    assert_eq!(o.shape(), &[1, l as i32, h as i32, d as i32]);
    let at = |x: &[f32], t: usize, head: usize, c: usize| x[(t * h + head) * d + c];
    let mut want = vec![0.0; l * h * d];
    for head in 0..h {
        for t in 0..l {
            let scores: Vec<f32> = (0..s)
                .map(|u| (0..d).map(|c| at(&q, t, head, c) * at(&k, u, head, c)).sum::<f32>() / (d as f32).sqrt())
                .collect();
            let top = scores.iter().cloned().fold(f32::MIN, f32::max);
            let e: Vec<f32> = scores.iter().map(|x| (x - top).exp()).collect();
            let z: f32 = e.iter().sum();
            for c in 0..d {
                want[(t * h + head) * d + c] = (0..s).map(|u| e[u] / z * at(&v, u, head, c)).sum();
            }
        }
    }
    assert!(max_err(&read(&o), &want) < MATMUL_F32);
}

#[test]
fn activations_match_their_formulas() {
    let x = values(32, 8);
    let a = Array::from_slice(&x, &[32]);
    let tanh_gelu: Vec<f32> = x
        .iter()
        .map(|v| 0.5 * v * (1.0 + ((2.0 / std::f32::consts::PI).sqrt() * (v + 0.044715 * v.powi(3))).tanh()))
        .collect();
    assert!(max_err(&read(&nn::in_f32(&a, mlx_rs::nn::gelu_approximate).unwrap()), &tanh_gelu) < 1e-5);
    let silu: Vec<f32> = x.iter().map(|v| v / (1.0 + (-v).exp())).collect();
    assert!(max_err(&read(&nn::in_f32(&a, mlx_rs::nn::silu).unwrap()), &silu) < 1e-6);
}

#[test]
fn weights_load_from_safetensors_by_name() {
    let dir = std::env::temp_dir().join(format!("raea-sys-mlx-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("w.safetensors");
    let w = Array::from_slice(&values(6, 9), &[2, 3]);
    Array::save_safetensors([("block.0.fc.weight", &w)], None, &path).unwrap();
    let loaded = Weights::load(&path);
    std::fs::remove_dir_all(&dir).unwrap();
    let loaded = loaded.unwrap();
    assert_eq!(loaded.count(), 6);
    assert_eq!(read(loaded.get("block.0.fc.weight").unwrap()), read(&w));
    assert!(loaded.get("block.1.fc.weight").unwrap_err().to_string().contains("block.1.fc.weight"));
}

#[test]
fn tensors_round_trip() {
    let t = Tensor::new(vec![2, 3], values(6, 10)).unwrap();
    assert_eq!(Tensor::from_array(&t.to_array()).unwrap(), t);
    assert!(Tensor::new(vec![2, 2], vec![0.0; 3]).is_err());
}
