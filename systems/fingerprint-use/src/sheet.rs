//! The sheet (FP3c, earshot's `sheet`): the whole piece in one PNG an agent or a person can read at a glance. Frames
//! along the top (a red frame where a moment is flagged), a lane per axis (grey for advice, amber plausible, red
//! strong, a white edge for gating), the sound's level, its spectrogram with flagged sound moments boxed, and the
//! checks' findings as ticks. Patch heatmaps on the frames come with FP12.

use crate::score::{Scored, clock};
use anyhow::Result;
use image::{Rgb, RgbImage};
use raea_sys_columns::media::{Rgb as Frame, SIDE};
use raea_sys_columns::window::Modality;

const W: u32 = 1600;
const LEFT: u32 = 150;
const LANE: u32 = 14;
const THUMBS: u32 = 12;
const SPEC: u32 = 120;

/// 5 x 7 glyphs, a row a byte (low 5 bits, left bit first)
const FONT: [(char, [u8; 7]); 44] = [
    ('A', [14, 17, 17, 31, 17, 17, 17]), ('B', [30, 17, 17, 30, 17, 17, 30]), ('C', [14, 17, 16, 16, 16, 17, 14]),
    ('D', [30, 17, 17, 17, 17, 17, 30]), ('E', [31, 16, 16, 30, 16, 16, 31]), ('F', [31, 16, 16, 30, 16, 16, 16]),
    ('G', [14, 17, 16, 23, 17, 17, 15]), ('H', [17, 17, 17, 31, 17, 17, 17]), ('I', [14, 4, 4, 4, 4, 4, 14]),
    ('J', [7, 2, 2, 2, 2, 18, 12]), ('K', [17, 18, 20, 24, 20, 18, 17]), ('L', [16, 16, 16, 16, 16, 16, 31]),
    ('M', [17, 27, 21, 21, 17, 17, 17]), ('N', [17, 17, 25, 21, 19, 17, 17]), ('O', [14, 17, 17, 17, 17, 17, 14]),
    ('P', [30, 17, 17, 30, 16, 16, 16]), ('Q', [14, 17, 17, 17, 21, 18, 13]), ('R', [30, 17, 17, 30, 20, 18, 17]),
    ('S', [15, 16, 16, 14, 1, 1, 30]), ('T', [31, 4, 4, 4, 4, 4, 4]), ('U', [17, 17, 17, 17, 17, 17, 14]),
    ('V', [17, 17, 17, 17, 17, 10, 4]), ('W', [17, 17, 17, 21, 21, 21, 10]), ('X', [17, 17, 10, 4, 10, 17, 17]),
    ('Y', [17, 17, 10, 4, 4, 4, 4]), ('Z', [31, 1, 2, 4, 8, 16, 31]), ('0', [14, 17, 19, 21, 25, 17, 14]),
    ('1', [4, 12, 4, 4, 4, 4, 14]), ('2', [14, 17, 1, 2, 4, 8, 31]), ('3', [31, 2, 4, 2, 1, 17, 14]),
    ('4', [2, 6, 10, 18, 31, 2, 2]), ('5', [31, 16, 30, 1, 1, 17, 14]), ('6', [6, 8, 16, 30, 17, 17, 14]),
    ('7', [31, 1, 2, 4, 8, 8, 8]), ('8', [14, 17, 17, 14, 17, 17, 14]), ('9', [14, 17, 17, 15, 1, 2, 12]),
    (':', [0, 12, 12, 0, 12, 12, 0]), ('.', [0, 0, 0, 0, 0, 12, 12]), ('-', [0, 0, 0, 31, 0, 0, 0]),
    ('/', [1, 1, 2, 4, 8, 16, 16]), ('(', [2, 4, 8, 8, 8, 4, 2]), (')', [8, 4, 2, 2, 2, 4, 8]),
    (',', [0, 0, 0, 0, 12, 4, 8]), ('%', [24, 25, 2, 4, 8, 19, 3]),
];

const INK: Rgb<u8> = Rgb([225, 225, 225]);
const DIM: Rgb<u8> = Rgb([70, 70, 74]);
const RED: Rgb<u8> = Rgb([230, 60, 50]);
const AMBER: Rgb<u8> = Rgb([235, 160, 40]);
const ADVICE: Rgb<u8> = Rgb([120, 120, 130]);

fn text(img: &mut RgbImage, x: u32, y: u32, s: &str, c: Rgb<u8>) {
    for (n, ch) in s.to_uppercase().chars().enumerate() {
        let Some((_, g)) = FONT.iter().find(|(k, _)| *k == ch) else { continue };
        for (r, bits) in g.iter().enumerate() {
            for b in 0..5 {
                let (px, py) = (x + n as u32 * 6 + b, y + r as u32);
                if bits >> (4 - b) & 1 == 1 && px < img.width() && py < img.height() {
                    img.put_pixel(px, py, c);
                }
            }
        }
    }
}

fn fill(img: &mut RgbImage, x0: u32, y0: u32, w: u32, h: u32, c: Rgb<u8>) {
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            img.put_pixel(x, y, c);
        }
    }
}

fn outline(img: &mut RgbImage, x0: u32, y0: u32, w: u32, h: u32, c: Rgb<u8>) {
    fill(img, x0, y0, w, 1, c);
    fill(img, x0, y0 + h.saturating_sub(1), w, 1, c);
    fill(img, x0, y0, 1, h, c);
    fill(img, x0 + w.saturating_sub(1), y0, 1, h, c);
}

/// A check's finding drawn as a tick: where, and what
pub struct Tick {
    pub a: f64,
    pub b: f64,
    pub what: String,
}

/// The sheet for a scored piece: `frames` are THUMBS frames spread over it (224 x 224 RGB), `pcm16` its sound
pub fn sheet(s: &Scored, frames: &[Frame], pcm16: &[f32], ticks: &[Tick], modality: &dyn Fn(&str) -> Option<Modality>, out: &std::path::Path) -> Result<()> {
    let axes: Vec<&str> = s.windows.first().map(|w| w.axes.iter().map(|a| a.axis.as_str()).collect()).unwrap_or_default();
    let span = W - LEFT - 10;
    let thumb = span / THUMBS;
    let h = 30 + thumb + 20 + axes.len() as u32 * LANE + 10 + 40 + 10 + SPEC + 10 + 16 + ticks.len().min(12) as u32 * 10 + 20;
    let mut img = RgbImage::from_pixel(W, h, Rgb([18, 18, 20]));
    let x_at = |t: f64| LEFT + ((t / s.dur.max(1e-6)) * span as f64).clamp(0.0, span as f64) as u32;
    text(&mut img, 10, 10, &format!("{} ({}), fingerprint {}: {} moments", s.file.rsplit('/').next().unwrap_or(""), clock(s.dur), s.fingerprint, s.moments.len()), INK);
    // frames
    let mut y = 30;
    for (i, f) in frames.iter().take(THUMBS as usize).enumerate() {
        let x0 = LEFT + i as u32 * thumb;
        for ty in 0..thumb - 2 {
            for tx in 0..thumb - 2 {
                let (sx, sy) = (tx as usize * SIDE / (thumb - 2) as usize, ty as usize * SIDE / (thumb - 2) as usize);
                let p = (sy * SIDE + sx) * 3;
                img.put_pixel(x0 + tx, y + ty, Rgb([f[p], f[p + 1], f[p + 2]]));
            }
        }
        let t = s.dur * (i as f64 + 0.5) / THUMBS as f64;
        if s.moments.iter().any(|m| m.a <= t && t <= m.b && modality(&m.axis).is_some_and(|m| !matches!(m, Modality::Sound | Modality::Events))) {
            outline(&mut img, x0, y, thumb - 2, thumb - 2, RED);
        }
    }
    y += thumb + 4;
    for k in 0..=10 {
        let t = s.dur * k as f64 / 10.0;
        fill(&mut img, x_at(t), y, 1, 4, DIM);
        text(&mut img, x_at(t).saturating_sub(10).max(LEFT), y + 6, &clock(t), DIM);
    }
    y += 20;
    // lanes
    for name in &axes {
        text(&mut img, 10, y + 3, name, INK);
        fill(&mut img, LEFT, y + 1, span, LANE - 2, Rgb([30, 30, 34]));
        for w in &s.windows {
            let Some(a) = w.axes.iter().find(|a| a.axis == *name).filter(|a| a.flagged) else { continue };
            let c = match (a.state.as_str(), a.band) {
                ("gating", Some("strong")) => RED,
                (_, Some("strong")) | (_, Some("plausible")) => AMBER,
                _ => ADVICE,
            };
            let (x0, x1) = (x_at(w.a + 0.5), x_at(w.b - 0.5).max(x_at(w.a + 0.5) + 1));
            fill(&mut img, x0, y + 1, x1 - x0, LANE - 2, c);
            if a.gating {
                outline(&mut img, x0, y + 1, x1 - x0, LANE - 2, INK);
            }
        }
        y += LANE;
    }
    // level
    y += 10;
    text(&mut img, 10, y + 16, "LEVEL", INK);
    for w in &s.windows {
        let v = ((w.level + 60.0) / 60.0).clamp(0.0, 1.0);
        let bar = (v * 38.0) as u32;
        let (x0, x1) = (x_at(w.a + 0.5), x_at(w.b - 0.5).max(x_at(w.a + 0.5) + 1));
        fill(&mut img, x0, y + 40 - bar, x1 - x0, bar, Rgb([80, 150, 220]));
    }
    y += 50;
    // spectrogram, with flagged sound moments boxed
    text(&mut img, 10, y + SPEC / 2, "SPECTRUM", INK);
    let spec = raea_sys_columns::media::spectrogram(pcm16, 16_000, span as usize, SPEC as usize);
    for c in 0..span as usize {
        for b in 0..SPEC as usize {
            let v = ((spec[c * SPEC as usize + b] + 90.0) / 80.0).clamp(0.0, 1.0);
            img.put_pixel(LEFT + c as u32, y + SPEC - 1 - b as u32, Rgb([(255.0 * v.powf(0.8)) as u8, (200.0 * v.powi(2)) as u8, (120.0 * v.powi(4) + 30.0 * v) as u8]));
        }
    }
    for m in s.moments.iter().filter(|m| modality(&m.axis).is_some_and(|m| matches!(m, Modality::Sound | Modality::Events))) {
        let (x0, x1) = (x_at(m.a), x_at(m.b));
        outline(&mut img, x0, y, (x1 - x0).max(2), SPEC, if m.gating { RED } else { AMBER });
    }
    y += SPEC + 10;
    // checks
    text(&mut img, 10, y, "CHECKS", INK);
    if ticks.is_empty() {
        text(&mut img, LEFT, y, "no findings", DIM);
    }
    for (i, t) in ticks.iter().take(12).enumerate() {
        let (x0, x1) = (x_at(t.a), x_at(t.b).max(x_at(t.a) + 2));
        fill(&mut img, x0, y, x1 - x0, 6, RED);
        text(&mut img, (x1 + 4).min(W - 300), y + 8 + i as u32 * 10, &format!("{} {}", clock(t.a), t.what), INK);
    }
    img.save(out)?;
    Ok(())
}
