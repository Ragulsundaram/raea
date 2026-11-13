//! Known defects put into real work: labels by construction for the mechanical axes (FP4a), and the cheats each axis
//! must still catch (FP4b). The same damage the 11-12 spike measured (docs/fingerprint.md FP0b, FP0d), on the 224 x 224
//! frames the encoder sees and on the sound at its own rate. Noise is seeded, so a bootstrap is the same run to run.

use crate::media::{Rgb, SIDE};
use image::{ImageBuffer, Rgb as Px, imageops};

/// The picture defects, then the sound defects, each with the axis it labels; cheats last
pub const PICTURE: [&str; 5] = ["blur", "dark", "glow", "flash", "freeze"];
pub const SOUND: [&str; 3] = ["noise", "dropout", "clip"];
/// Each cheat and the defect it hides: a de-rez sharpened after, glow at half strength, noise at a third of the level,
/// a freeze with a slow zoom on it, a picture darkened half as much, a white flash where the defect is black, a gap a
/// third as long, and clipping a third as hard
pub const CHEATS: [(&str, &str); 8] = [
    ("sharpblur", "blur"),
    ("glow_half", "glow"),
    ("noise_low", "noise"),
    ("freeze_zoom", "freeze"),
    ("dark_half", "dark"),
    ("flash_white", "flash"),
    ("dropout_short", "dropout"),
    ("clip_soft", "clip"),
];

type Img = ImageBuffer<Px<u8>, Vec<u8>>;

fn img(f: &Rgb) -> Img {
    Img::from_raw(SIDE as u32, SIDE as u32, f.clone()).expect("a 224 x 224 frame")
}

fn soft(f: &Rgb) -> Img {
    let small = imageops::resize(&img(f), (SIDE / 5) as u32, (SIDE / 5) as u32, imageops::FilterType::Triangle);
    imageops::resize(&small, SIDE as u32, SIDE as u32, imageops::FilterType::Triangle)
}

fn glow(f: &Rgb, k: f32) -> Rgb {
    let g = imageops::blur(&img(f), SIDE as f32 / 22.0);
    f.iter().zip(g.as_raw()).map(|(&x, &b)| (255.0 * (1.0 - (1.0 - x as f32 / 255.0) * (1.0 - k * b as f32 / 255.0))).clamp(0.0, 255.0) as u8).collect()
}

/// An unsharp mask (radius 4, 250%, threshold 2) over the de-rez: crisp edges with the detail still gone
fn sharpened(f: &Rgb) -> Rgb {
    let s = soft(f);
    let b = imageops::blur(&s, 4.0);
    s.as_raw().iter().zip(b.as_raw()).map(|(&x, &y)| {
        let d = x as f32 - y as f32;
        if d.abs() < 2.0 { x } else { (x as f32 + 2.5 * d).clamp(0.0, 255.0) as u8 }
    }).collect()
}

/// The first frame held, pushed in 1% a frame from its centre
fn zoomed(f: &Rgb, i: usize) -> Rgb {
    let side = (SIDE as f32 / (1.0 + 0.01 * i as f32)).round() as u32;
    let at = (SIDE as u32 - side) / 2;
    let crop = imageops::crop_imm(&img(f), at, at, side, side).to_image();
    imageops::resize(&crop, SIDE as u32, SIDE as u32, imageops::FilterType::Triangle).into_raw()
}

/// A window's frames with a picture defect (or cheat) put in
/// A frame with every channel scaled by `k`
fn darker(f: &Rgb, k: f32) -> Rgb {
    f.iter().map(|&x| (x as f32 * k) as u8).collect()
}

/// Sound driven `gain` times louder into a hard ceiling at `ceil`
fn clipped(w: &mut [f32], gain: f32, ceil: f32) {
    w.iter_mut().for_each(|x| *x = (*x * gain).clamp(-ceil, ceil));
}

pub fn picture(frames: &[Rgb], kind: &str) -> Vec<Rgb> {
    match kind {
        "blur" => frames.iter().map(|f| soft(f).into_raw()).collect(),
        "sharpblur" => frames.iter().map(sharpened).collect(),
        "glow" => frames.iter().map(|f| glow(f, 0.7)).collect(),
        "glow_half" => frames.iter().map(|f| glow(f, 0.35)).collect(),
        "dark" => frames.iter().map(|f| darker(f, 0.3)).collect(),
        "dark_half" => frames.iter().map(|f| darker(f, 0.65)).collect(),
        "freeze" => frames.iter().map(|_| frames[0].clone()).collect(),
        "freeze_zoom" => (0..frames.len()).map(|i| zoomed(&frames[0], i)).collect(),
        "flash" | "flash_white" => {
            let mut v = frames.to_vec();
            if !v.is_empty() {
                let m = v.len() / 2;
                v[m] = vec![if kind == "flash" { 0 } else { 255 }; SIDE * SIDE * 3];
            }
            v
        }
        _ => frames.to_vec(),
    }
}

/// FP0f's local defects, each in one box of the frame, with the picture axis it labels: a blurred patch, a black
/// hole, a flat grey surface, heavy grain (corrupted, as a glitch) and a local glow. FP12's patch heads learn where.
pub const LOCAL: [(&str, &str); 5] = [("blur_box", "blur"), ("hole", "glitch"), ("flat", "barren"), ("grain", "glitch"), ("glow_box", "glow")];

/// A box in a 224 x 224 frame: its left, top and side, in pixels
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Square {
    pub x: usize,
    pub y: usize,
    pub side: usize,
}

impl Square {
    /// A random box 64-80 px a side, wholly in the frame (FP0f's)
    pub fn random(rng: &mut fastrand::Rng) -> Square {
        let side = rng.usize(64..=80);
        Square { x: rng.usize(0..=SIDE - side), y: rng.usize(0..=SIDE - side), side }
    }
    /// The pixels inside it, as indices into a 224 x 224 frame
    pub fn pixels(&self) -> Vec<usize> {
        (self.y..self.y + self.side).flat_map(|y| (self.x..self.x + self.side).map(move |x| y * SIDE + x)).collect()
    }
}

/// A frame with a local defect put into one box; the rest of the frame is untouched
pub fn local(f: &Rgb, kind: &str, b: Square, noise: &mut Noise) -> Rgb {
    let whole: Rgb = match kind {
        "blur_box" => soft(f).into_raw(),
        "glow_box" => glow(f, 0.9),
        _ => f.clone(),
    };
    let inside = b.pixels();
    let grey = (inside.iter().map(|&i| f[i * 3..i * 3 + 3].iter().map(|&c| c as f32).sum::<f32>() / 3.0).sum::<f32>() / inside.len().max(1) as f32) as u8;
    let mut out = f.clone();
    for &i in &inside {
        for c in 0..3 {
            out[i * 3 + c] = match kind {
                "hole" => 0,
                "flat" => grey,
                "grain" => (f[i * 3 + c] as f32 + noise.normal() * 60.0).clamp(0.0, 255.0) as u8,
                _ => whole[i * 3 + c],
            };
        }
    }
    out
}

/// A seeded normal source (fastrand's, Box-Muller)
pub struct Noise(fastrand::Rng);

impl Noise {
    pub fn new(seed: u64) -> Noise {
        Noise(fastrand::Rng::with_seed(seed))
    }
    pub fn normal(&mut self) -> f32 {
        let (u, v) = (self.0.f64().max(1e-12), self.0.f64());
        ((-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()) as f32
    }
}

/// A window's sound (at `rate`) with a sound defect (or cheat) put in
pub fn sound(w: &[f32], kind: &str, rate: u32, noise: &mut Noise) -> Vec<f32> {
    let mut w = w.to_vec();
    let rms = (w.iter().map(|x| x * x).sum::<f32>() / w.len().max(1) as f32 + 1e-9).sqrt();
    let mut add = |w: &mut Vec<f32>, k: f32| w.iter_mut().for_each(|x| *x += noise.normal() * rms.max(0.02) * k);
    match kind {
        "noise" => add(&mut w, 0.5),
        "noise_low" => add(&mut w, 0.15),
        "dropout" | "dropout_short" => {
            let gap = if kind == "dropout" { 0.6 } else { 0.2 };
            let a = (w.len() / 2).saturating_sub((gap / 2.0 * rate as f64) as usize);
            let b = (a + (gap * rate as f64) as usize).min(w.len());
            w[a..b].iter_mut().for_each(|x| *x = 0.0);
        }
        "clip" => clipped(&mut w, 10.0, 0.3),
        "clip_soft" => clipped(&mut w, 3.0, 0.6),
        _ => {}
    }
    w
}
