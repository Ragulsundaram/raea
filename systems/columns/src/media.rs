//! A file's frames and sound the way the encoders read them: frames squashed to 224 x 224 RGB (as SigLIP's processor
//! does, bicubic), sound as mono floats at the encoder's rate (raea's band-limited resampler). Decoding goes through
//! the media plane's preferred backends (VideoToolbox on Apple, then the portable decoders, then the ffmpeg CLI).

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub const SIDE: usize = 224;
/// A frame as SIDE x SIDE RGB8, row-major
pub type Rgb = Vec<u8>;

/// A file's sha256, hex
pub fn sha256(file: &Path) -> Result<String> {
    let mut f = std::fs::File::open(file).map_err(|e| anyhow!("{}: {e}", file.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// The file's picture, opened by the first backend that can (also how a mark is grown into its thing, FP11b)
pub fn video(file: &Path) -> Result<Box<dyn raea_io_media::VideoDecoder>> {
    let mut why = Vec::new();
    for b in raea_io_media::preferred() {
        match b.open_video(file) {
            Ok(d) => return Ok(d),
            Err(e) => why.push(format!("{}: {e}", b.name())),
        }
    }
    Err(anyhow!("{}: no decoder opened it ({})", file.display(), why.join("; ")))
}

/// How long the file is, seconds: its picture's length, or its sound's when it has no picture
pub fn duration(file: &Path) -> Result<f64> {
    match video(file) {
        Ok(d) => Ok(d.info().duration),
        Err(e) => {
            let s = sound(file, 16_000).map_err(|_| e)?;
            Ok(s.len() as f64 / 16_000.0)
        }
    }
}

/// Frames at `fps` from `start` for `dur` seconds (t = start + i / fps), each squashed to SIDE x SIDE. A file with no
/// picture gives none.
pub fn frames(file: &Path, fps: f64, start: f64, dur: f64) -> Result<Vec<Rgb>> {
    let Ok(mut d) = video(file) else { return Ok(Vec::new()) };
    let n = (dur * fps).floor().max(1.0) as usize;
    let end = d.info().duration;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = start + i as f64 / fps;
        if t >= end && i > 0 {
            break;
        }
        let f = d.frame_at(t.min((end - 1e-3).max(0.0))).map_err(|e| anyhow!("{}: frame at {t:.2} s: {e}", file.display()))?;
        out.push(squash(&f.rgba, f.width as usize, f.height as usize));
    }
    Ok(out)
}

/// RGBA8 at w x h to SIDE x SIDE RGB8, bicubic (Catmull-Rom)
pub fn squash(rgba: &[u8], w: usize, h: usize) -> Rgb {
    let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, rgba.to_vec()) else { return vec![0; SIDE * SIDE * 3] };
    let small = image::imageops::resize(&img, SIDE as u32, SIDE as u32, image::imageops::FilterType::CatmullRom);
    small.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect()
}

/// The file's sound as mono at `rate`; empty when it has none
pub fn sound(file: &Path, rate: u32) -> Result<Vec<f32>> {
    for b in raea_io_media::preferred() {
        let Ok(a) = b.decode_audio(file) else { continue };
        if a.channels.is_empty() || a.len() == 0 {
            continue;
        }
        let n = a.len();
        let k = a.channels.len() as f32;
        // the channels' average
        let mut mono = vec![0f32; n];
        for c in &a.channels {
            for (m, x) in mono.iter_mut().zip(c) {
                *m += x / k;
            }
        }
        let m = raea_io_media::soft_resample(&raea_io_media::Audio { rate: a.rate, channels: vec![mono] }, rate);
        return Ok(m.channels.into_iter().next().unwrap_or_default());
    }
    Ok(Vec::new())
}

/// RMS level in dBFS per 0.1 s (-100 for digital silence)
pub fn levels(pcm: &[f32], rate: u32) -> Vec<f32> {
    let step = (rate / 10).max(1) as usize;
    pcm.chunks(step).map(|c| (10.0 * (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).max(1e-10).log10()).max(-100.0)).collect()
}

/// A spectrogram of mono sound for the sheet: `cols` frames of 512-sample FFTs spread over the sound, each folded to
/// `bins` bands (log-spaced from 50 Hz), in dB; [cols][bins]
pub fn spectrogram(pcm: &[f32], rate: u32, cols: usize, bins: usize) -> Vec<f32> {
    const N: usize = 512;
    let fft = rustfft::FftPlanner::<f32>::new().plan_fft_forward(N);
    let mut out = vec![-100f32; cols * bins];
    if pcm.len() < N || cols == 0 {
        return out;
    }
    let (lo, hi) = (50f32.ln(), (rate as f32 / 2.0).ln());
    for c in 0..cols {
        let at = (pcm.len() - N) * c / cols.max(2).saturating_sub(1).max(1);
        let mut buf: Vec<rustfft::num_complex::Complex<f32>> = (0..N)
            .map(|i| {
                let hann = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / N as f32).cos();
                rustfft::num_complex::Complex::new(pcm[at + i] * hann, 0.0)
            })
            .collect();
        fft.process(&mut buf);
        let mut band = vec![0f32; bins];
        for (k, z) in buf.iter().take(N / 2).enumerate().skip(1) {
            let hz = k as f32 * rate as f32 / N as f32;
            let b = (((hz.ln() - lo) / (hi - lo)) * bins as f32).floor();
            if (0.0..bins as f32).contains(&b) {
                band[b as usize] += z.norm_sqr();
            }
        }
        for (b, e) in band.iter().enumerate() {
            out[c * bins + b] = 10.0 * (e + 1e-10).log10();
        }
    }
    out
}
