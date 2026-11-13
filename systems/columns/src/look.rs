//! The look column: what SigLIP's pooled vector is built to ignore (FP0d: glow at half strength caught 0.20). Per
//! frame, on the 224 x 224 picture the encoder sees, all in 0..1 units: an 8-bin luminance histogram, mean and spread
//! of luminance, mean saturation, local contrast (mean distance from the 3 x 3 neighbourhood's mean), sharpness (the
//! Laplacian's spread), the share of near-black and near-white pixels, and the 5th-95th percentile range.

use crate::media::SIDE;

pub const LOOK: usize = 16;

/// One frame's look, LOOK values
pub fn look(rgb: &[u8]) -> [f32; LOOK] {
    let n = SIDE * SIDE;
    let mut y = vec![0f32; n];
    let mut sat = 0f64;
    for i in 0..n {
        let (r, g, b) = (rgb[i * 3] as f32 / 255.0, rgb[i * 3 + 1] as f32 / 255.0, rgb[i * 3 + 2] as f32 / 255.0);
        y[i] = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
        sat += if hi > 1e-3 { ((hi - lo) / hi) as f64 } else { 0.0 };
    }
    let mut o = [0f32; LOOK];
    for v in &y {
        o[((v * 8.0) as usize).min(7)] += 1.0 / n as f32;
    }
    let mean = y.iter().sum::<f32>() / n as f32;
    o[8] = mean;
    o[9] = (y.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n as f32).sqrt();
    o[10] = (sat / n as f64) as f32;
    let (mut local, mut lap, mut lap2, mut inner) = (0f64, 0f64, 0f64, 0f64);
    for r in 1..SIDE - 1 {
        for c in 1..SIDE - 1 {
            let i = r * SIDE + c;
            let around = [i - SIDE - 1, i - SIDE, i - SIDE + 1, i - 1, i + 1, i + SIDE - 1, i + SIDE, i + SIDE + 1];
            let box_mean = (around.iter().map(|&j| y[j]).sum::<f32>() + y[i]) / 9.0;
            local += (y[i] - box_mean).abs() as f64;
            let l = (y[i - SIDE] + y[i + SIDE] + y[i - 1] + y[i + 1] - 4.0 * y[i]) as f64;
            lap += l;
            lap2 += l * l;
            inner += 1.0;
        }
    }
    o[11] = (local / inner) as f32 * 10.0;
    o[12] = ((lap2 / inner - (lap / inner).powi(2)).max(0.0).sqrt()) as f32 * 10.0;
    o[13] = y.iter().filter(|v| **v < 0.1).count() as f32 / n as f32;
    o[14] = y.iter().filter(|v| **v > 0.9).count() as f32 / n as f32;
    let mut s = y;
    s.sort_by(f32::total_cmp);
    o[15] = s[n * 95 / 100] - s[n * 5 / 100];
    o
}
