//! CLAP htsat-unfused's audio tower on ONNX Runtime (Xenova's q8 export, Apache-2.0), with its front end written out
//! here the way its feature extractor computes it (transformers' ClapFeatureExtractor, "rand_trunc" with
//! "repeatpad"): 48 kHz mono padded to 10 s by whole repeats while one more fits, then zeros; a 1024-point periodic
//! Hann STFT, hop 480, centred with reflect padding (1001 frames); power through a 64-band slaney mel bank (50 Hz to
//! 14 kHz, slaney-normalised); 10 log10 of each band, floored at 1e-10. The tower sees [1, 1, 1001, 64].

use crate::siglip::{Input, run};
use anyhow::{Context, Result};
use ort::session::Session;
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::path::Path;
use std::sync::Arc;

pub const RATE: u32 = 48_000;
const MAX: usize = 480_000;
const NFFT: usize = 1024;
const HOP: usize = 480;
const MELS: usize = 64;
const BINS: usize = NFFT / 2 + 1;
const FRAMES: usize = 1 + MAX / HOP;

pub struct Audio {
    s: Session,
    bank: Vec<f64>,
    /// each band's first and last bin with weight (the bank is mostly zeros)
    span: Vec<(usize, usize)>,
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
}

fn slaney_mel(f: f64) -> f64 {
    let logstep = 27.0 / 6.4f64.ln();
    if f >= 1000.0 { 15.0 + (f / 1000.0).ln() * logstep } else { 3.0 * f / 200.0 }
}

fn slaney_hz(m: f64) -> f64 {
    let logstep = 6.4f64.ln() / 27.0;
    if m >= 15.0 { 1000.0 * (logstep * (m - 15.0)).exp() } else { 200.0 * m / 3.0 }
}

/// The mel bank, [MELS][BINS]
pub(crate) fn bank() -> Vec<f64> {
    let (lo, hi) = (slaney_mel(50.0), slaney_mel(14_000.0));
    // MELS + 2 edges evenly spaced in mels, and each FFT bin's centre in Hz
    let step = (hi - lo) / (MELS + 1) as f64;
    let mut edges = Vec::with_capacity(MELS + 2);
    for i in 0..MELS + 2 {
        edges.push(slaney_hz(lo + step * i as f64));
    }
    let bin = (RATE / 2) as f64 / (BINS - 1) as f64;
    let mut fft = Vec::with_capacity(BINS);
    for j in 0..BINS {
        fft.push(bin * j as f64);
    }
    let mut out = vec![0f64; MELS * BINS];
    for i in 0..MELS {
        let enorm = 2.0 / (edges[i + 2] - edges[i]);
        for (j, &f) in fft.iter().enumerate() {
            let down = (f - edges[i]) / (edges[i + 1] - edges[i]);
            let up = (edges[i + 2] - f) / (edges[i + 2] - edges[i + 1]);
            out[i * BINS + j] = down.min(up).max(0.0) * enorm;
        }
    }
    out
}

/// The clip as the tower reads it: padded to MAX samples by repeats, then zeros (a longer clip keeps its first MAX)
pub(crate) fn padded(x: &[f32]) -> Vec<f64> {
    let mut p = vec![0f64; MAX];
    let n = x.len().min(MAX);
    for (d, s) in p.iter_mut().zip(&x[..n]) {
        *d = *s as f64;
    }
    if n > 0 && n < MAX {
        let mut i = n;
        while i < MAX - n {
            for k in 0..n {
                p[i + k] = x[k] as f64;
            }
            i += n;
        }
    }
    p
}

impl Audio {
    pub fn load(file: &Path) -> Result<Audio> {
        let window = (0..NFFT).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / NFFT as f64).cos()).collect();
        let fft = FftPlanner::new().plan_fft_forward(NFFT);
        let bank = bank();
        let span = (0..MELS)
            .map(|m| {
                let row = &bank[m * BINS..(m + 1) * BINS];
                let lo = row.iter().position(|x| *x > 0.0).unwrap_or(0);
                (lo, row.iter().rposition(|x| *x > 0.0).map_or(lo, |h| h + 1))
            })
            .collect();
        Ok(Audio { s: raea_sys_ml::session(file).context("CLAP's audio tower")?, bank, span, window, fft })
    }

    /// The log-mel features, [FRAMES][MELS]
    pub fn features(&self, x: &[f32]) -> Vec<f32> {
        let w = padded(x);
        let half = NFFT / 2;
        // reflect padding: w[1..=half] mirrored before, w[MAX-1-half..MAX-1] mirrored after
        let at = |i: isize| -> f64 {
            let n = MAX as isize;
            let j = if i < 0 { -i } else if i >= n { 2 * (n - 1) - i } else { i };
            w[j as usize]
        };
        let mut out = vec![0f32; FRAMES * MELS];
        let mut buf = vec![Complex::new(0.0, 0.0); NFFT];
        // power and the mel sums in single precision, as transformers.js keeps them (its magnitudes are a Float32Array)
        let mut power = vec![0f32; BINS];
        for f in 0..FRAMES {
            let start = (f * HOP) as isize - half as isize;
            for (j, b) in buf.iter_mut().enumerate() {
                *b = Complex::new(at(start + j as isize) * self.window[j], 0.0);
            }
            self.fft.process(&mut buf);
            for (j, p) in power.iter_mut().enumerate() {
                *p = buf[j].norm_sqr() as f32;
            }
            for m in 0..MELS {
                let (lo, hi) = self.span[m];
                let e: f32 = self.bank[m * BINS + lo..m * BINS + hi].iter().zip(&power[lo..hi]).map(|(a, b)| *a as f32 * b).sum();
                out[f * MELS + m] = (10.0 * (e as f64).max(1e-10).log10()) as f32;
            }
        }
        out
    }

    /// Each clip's vector, L2-normalised: [clips][512]
    pub fn embed(&mut self, clips: &[&[f32]]) -> Result<Vec<f32>> {
        let mut v = Vec::with_capacity(clips.len() * crate::CLAP);
        for c in clips {
            let x = self.features(c);
            v.extend(run(&mut self.s, ("input_features", vec![1, 1, FRAMES as i64, MELS as i64], Input::F32(x)), &["audio_embeds"])?.remove(0));
        }
        crate::normalise(&mut v, crate::CLAP);
        Ok(v)
    }
}
