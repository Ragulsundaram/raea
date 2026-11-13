//! A head (FP2c): logistic regression on one modality's standardised features, L2-regularised (C = 0.5) with balanced
//! class weights, fit by L-BFGS in milliseconds on the CPU; Bradley-Terry for pairs is the same fit on the pair's
//! difference with no intercept. Also the measures every report uses: AUC and the band thresholds.

use serde::{Deserialize, Serialize};

/// Feature statistics: the encoder's own, from unlabelled windows of the person's work (FP0b: unscaled, SigLIP hid
/// behind YAMNet's larger values)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scaler {
    pub mean: Vec<f32>,
    pub std: Vec<f32>,
}

impl Scaler {
    pub fn fit(rows: &[&[f32]]) -> Scaler {
        let d = rows.first().map_or(0, |r| r.len());
        let n = rows.len().max(1) as f64;
        let mut mean = vec![0f64; d];
        let mut sq = vec![0f64; d];
        for r in rows {
            for (k, &x) in r.iter().enumerate() {
                mean[k] += x as f64 / n;
                sq[k] += (x as f64).powi(2) / n;
            }
        }
        let std = (0..d).map(|k| ((sq[k] - mean[k].powi(2)).max(0.0).sqrt() as f32).max(1e-6)).collect();
        Scaler { mean: mean.into_iter().map(|m| m as f32).collect(), std }
    }
    pub fn apply(&self, x: &[f32]) -> Vec<f64> {
        x.iter().zip(&self.mean).zip(&self.std).map(|((x, m), s)| ((x - m) / s) as f64).collect()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Logistic {
    pub scaler: Scaler,
    pub w: Vec<f32>,
    pub b: f32,
}

impl Logistic {
    /// The decision value (log-odds) for raw features
    pub fn score(&self, x: &[f32]) -> f32 {
        let z = self.scaler.apply(x);
        (z.iter().zip(&self.w).map(|(a, b)| a * *b as f64).sum::<f64>() + self.b as f64) as f32
    }

    /// Fit on raw rows (`y` true for the defect), with the given scaler; C as scikit-learn's
    pub fn fit(scaler: Scaler, x: &[&[f32]], y: &[bool], weights: Option<&[f32]>, c: f64, intercept: bool) -> Logistic {
        let z: Vec<Vec<f64>> = x.iter().map(|r| scaler.apply(r)).collect();
        let (pos, neg) = (y.iter().filter(|v| **v).count().max(1) as f64, y.iter().filter(|v| !**v).count().max(1) as f64);
        let n = y.len() as f64;
        // balanced: each class weighs n / (2 * its count), times the row's own weight
        let cw: Vec<f64> = y.iter().enumerate().map(|(i, &v)| (if v { n / (2.0 * pos) } else { n / (2.0 * neg) }) * weights.map_or(1.0, |w| w[i] as f64)).collect();
        let d = z.first().map_or(0, |r| r.len());
        let f = |p: &[f64], g: &mut [f64]| -> f64 {
            g.iter_mut().for_each(|v| *v = 0.0);
            let (w, b) = (&p[..d], if intercept { p[d] } else { 0.0 });
            let mut loss = 0.0;
            for (i, r) in z.iter().enumerate() {
                let s = r.iter().zip(w).map(|(a, b)| a * b).sum::<f64>() + b;
                let t = if y[i] { 1.0 } else { -1.0 };
                let m = t * s;
                loss += cw[i] * if m > 0.0 { (-m).exp().ln_1p() } else { -m + m.exp().ln_1p() };
                let dl = -cw[i] * t / (1.0 + m.exp());
                for k in 0..d {
                    g[k] += dl * r[k];
                }
                if intercept {
                    g[d] += dl;
                }
            }
            // the penalty: |w|^2 / 2, the data term times C (scikit-learn's form)
            for k in 0..d {
                g[k] = c * g[k] + w[k];
            }
            if intercept {
                g[d] *= c;
            }
            c * loss + 0.5 * w.iter().map(|v| v * v).sum::<f64>()
        };
        let p = lbfgs(vec![0.0; d + usize::from(intercept)], f, 300);
        Logistic { scaler, w: p[..d].iter().map(|v| *v as f32).collect(), b: if intercept { p[d] as f32 } else { 0.0 } }
    }
}

/// Limited-memory BFGS (10 pairs) with a backtracking line search
fn lbfgs(mut x: Vec<f64>, f: impl Fn(&[f64], &mut [f64]) -> f64, iters: usize) -> Vec<f64> {
    let n = x.len();
    let mut g = vec![0.0; n];
    let mut fx = f(&x, &mut g);
    let (mut s_hist, mut y_hist): (Vec<Vec<f64>>, Vec<Vec<f64>>) = (vec![], vec![]);
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    for _ in 0..iters {
        let gn = dot(&g, &g).sqrt();
        if gn < 1e-6 * (1.0 + fx.abs()) {
            break;
        }
        // two-loop recursion
        let mut q = g.clone();
        let mut alpha = vec![0.0; s_hist.len()];
        for i in (0..s_hist.len()).rev() {
            let rho = 1.0 / dot(&y_hist[i], &s_hist[i]);
            alpha[i] = rho * dot(&s_hist[i], &q);
            q.iter_mut().zip(&y_hist[i]).for_each(|(a, b)| *a -= alpha[i] * b);
        }
        if let (Some(s), Some(y)) = (s_hist.last(), y_hist.last()) {
            let gamma = dot(s, y) / dot(y, y);
            q.iter_mut().for_each(|v| *v *= gamma);
        } else {
            q.iter_mut().for_each(|v| *v /= gn.max(1.0));
        }
        for i in 0..s_hist.len() {
            let rho = 1.0 / dot(&y_hist[i], &s_hist[i]);
            let beta = rho * dot(&y_hist[i], &q);
            q.iter_mut().zip(&s_hist[i]).for_each(|(a, b)| *a += (alpha[i] - beta) * b);
        }
        let dir: Vec<f64> = q.iter().map(|v| -v).collect();
        let slope = dot(&g, &dir);
        if slope >= 0.0 {
            s_hist.clear();
            y_hist.clear();
            continue;
        }
        let mut step = 1.0;
        let mut g2 = vec![0.0; n];
        let mut x2;
        let mut f2;
        loop {
            x2 = x.iter().zip(&dir).map(|(a, b)| a + step * b).collect::<Vec<_>>();
            f2 = f(&x2, &mut g2);
            if f2 <= fx + 1e-4 * step * slope || step < 1e-10 {
                break;
            }
            step *= 0.5;
        }
        let s: Vec<f64> = x2.iter().zip(&x).map(|(a, b)| a - b).collect();
        let yv: Vec<f64> = g2.iter().zip(&g).map(|(a, b)| a - b).collect();
        if dot(&s, &yv) > 1e-12 {
            s_hist.push(s);
            y_hist.push(yv);
            if s_hist.len() > 10 {
                s_hist.remove(0);
                y_hist.remove(0);
            }
        }
        let done = (fx - f2).abs() < 1e-9 * (1.0 + fx.abs());
        x = x2;
        g = g2;
        fx = f2;
        if done {
            break;
        }
    }
    x
}

/// Area under the ROC curve: the chance a random defect scores above a random clean one (ties count half)
pub fn auc(pos: &[f32], neg: &[f32]) -> f64 {
    if pos.is_empty() || neg.is_empty() {
        return f64::NAN;
    }
    let mut all: Vec<(f32, bool)> = pos.iter().map(|&s| (s, true)).chain(neg.iter().map(|&s| (s, false))).collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut rank_sum, mut i) = (0.0, 0);
    while i < all.len() {
        let mut j = i;
        while j < all.len() && all[j].0 == all[i].0 {
            j += 1;
        }
        let r = (i + j + 1) as f64 / 2.0;
        rank_sum += r * all[i..j].iter().filter(|x| x.1).count() as f64;
        i = j;
    }
    let (p, n) = (pos.len() as f64, neg.len() as f64);
    (rank_sum - p * (p + 1.0) / 2.0) / (p * n)
}

/// The score above which `q` of the clean scores fall (the threshold that flags 1 - q of clean work)
pub fn quantile(v: &[f32], q: f64) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    if s.is_empty() {
        return f32::INFINITY;
    }
    let x = q * (s.len() - 1) as f64;
    let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (x - lo as f64) as f32
}

/// The lowest threshold at which a flag is right `precision` of the time on held-out scores (earshot's bands: strong
/// 0.9, plausible 0.7); None when no threshold reaches it
pub fn band(pos: &[f32], neg: &[f32], precision: f64) -> Option<f32> {
    let mut cands: Vec<f32> = pos.to_vec();
    cands.sort_by(f32::total_cmp);
    cands.into_iter().find(|&t| {
        let tp = pos.iter().filter(|s| **s >= t).count() as f64;
        let fp = neg.iter().filter(|s| **s >= t).count() as f64;
        tp >= 3.0 && tp / (tp + fp) >= precision
    })
}
