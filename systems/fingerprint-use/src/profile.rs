//! Profiles (FP6e): ranges taken from work the person admires, and a piece critiqued against them (earshot's `profile`
//! and `critique`). Each measure (loudness and its range, spectral tilt, cut rate, motion, brightness, contrast,
//! saturation, sharpness) gets the range the references span; a piece is told which measures fall outside it, where,
//! and which way to move. A profile is checked leave-one-out: each reference against a profile of the others must fall
//! in range more often than against unrelated work, or it's reported as not telling this taste apart.

use raea_sys_columns::{Columns, FPS, SIG};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Measures = BTreeMap<String, f32>;

/// What a measure is, in words, for the critique
pub const NAMES: [(&str, &str); 9] = [
    ("loudness", "loudness (dB)"),
    ("loudness_range", "the range between quiet and loud (dB)"),
    ("tilt", "bass against treble (dB)"),
    ("cuts_per_min", "cuts a minute"),
    ("motion", "how much the picture moves"),
    ("brightness", "brightness"),
    ("contrast", "contrast"),
    ("saturation", "colour saturation"),
    ("sharpness", "sharpness"),
];

/// The average; nothing averages to 0
fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

fn pct(v: &[f32], q: f32) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    s.get(((s.len().max(1) - 1) as f32 * q) as usize).copied().unwrap_or(0.0)
}

/// Frame-to-frame SigLIP distances over [a, b) seconds
fn steps(c: &Columns, a: f64, b: f64) -> Vec<f32> {
    let n = c.sig.len() / SIG;
    let (fa, fb) = (((a * FPS) as usize).min(n), ((b * FPS) as usize).min(n));
    (fa.max(1)..fb).map(|i| {
        let (x, y) = (&c.sig[(i - 1) * SIG..i * SIG], &c.sig[i * SIG..(i + 1) * SIG]);
        let d: f32 = x.iter().zip(y).map(|(p, q)| p * q).sum();
        let nx = x.iter().map(|p| p * p).sum::<f32>().sqrt() * y.iter().map(|q| q * q).sum::<f32>().sqrt();
        1.0 - d / nx.max(1e-6)
    }).collect()
}

/// A distance this far is a cut
const CUT: f32 = 0.25;

/// The measures over [a, b) seconds of a piece: its columns, and its spectrogram (`spec`, `bins` bands a column, one
/// column a tenth of a second, from raea_sys_columns::media::spectrogram) for the tilt
pub fn measures(c: &Columns, spec: &[f32], bins: usize, a: f64, b: f64) -> Measures {
    let lv: Vec<f32> = c.level.iter().enumerate().filter(|(i, _)| (*i as f64 / 10.0) >= a && (*i as f64 / 10.0) < b).map(|(_, v)| *v).filter(|v| *v > -60.0).collect();
    let look = raea_sys_columns::look::LOOK;
    let n = c.look.len() / look;
    let (fa, fb) = (((a * FPS) as usize).min(n), ((b * FPS) as usize).min(n));
    let col = |k: usize| mean(&(fa..fb).map(|i| c.look[i * look + k]).collect::<Vec<_>>());
    let st = steps(c, a, b);
    let cols = spec.len() / bins.max(1);
    let (sa, sb) = (((a * 10.0) as usize).min(cols), ((b * 10.0) as usize).min(cols));
    let band = |r: std::ops::Range<usize>| mean(&(sa..sb).flat_map(|i| r.clone().map(move |k| (i, k))).map(|(i, k)| spec[i * bins + k]).collect::<Vec<_>>());
    let minutes = ((b - a) / 60.0).max(1e-3) as f32;
    let mut m = Measures::new();
    m.insert("loudness".into(), mean(&lv));
    m.insert("loudness_range".into(), pct(&lv, 0.9) - pct(&lv, 0.1));
    m.insert("tilt".into(), band(0..bins / 3) - band(bins * 2 / 3..bins));
    m.insert("cuts_per_min".into(), st.iter().filter(|d| **d > CUT).count() as f32 / minutes);
    m.insert("motion".into(), mean(&st.iter().copied().filter(|d| *d <= CUT).collect::<Vec<_>>()));
    m.insert("brightness".into(), col(8));
    m.insert("contrast".into(), col(11));
    m.insert("saturation".into(), col(10));
    m.insert("sharpness".into(), col(12));
    m
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    /// each reference's file and its measures
    pub refs: Vec<(String, Measures)>,
    /// each measure's range over the references
    pub ranges: BTreeMap<String, (f32, f32)>,
    /// leave-one-out: share of a reference's measures inside the others' ranges, and of unrelated work's
    pub inside_own: Option<f32>,
    pub inside_other: Option<f32>,
    pub tells: Option<bool>,
}

/// The ranges the references span, widened by a tenth of their width each way
pub fn ranges(refs: &[&Measures]) -> BTreeMap<String, (f32, f32)> {
    let mut out = BTreeMap::new();
    for (k, _) in NAMES {
        let v: Vec<f32> = refs.iter().filter_map(|m| m.get(k).copied()).collect();
        if v.is_empty() {
            continue;
        }
        let (lo, hi) = (v.iter().copied().fold(f32::MAX, f32::min), v.iter().copied().fold(f32::MIN, f32::max));
        let pad = (hi - lo) * 0.1;
        out.insert(k.to_string(), (lo - pad, hi + pad));
    }
    out
}

fn inside(m: &Measures, r: &BTreeMap<String, (f32, f32)>) -> f32 {
    let hits: Vec<bool> = r.iter().filter_map(|(k, (lo, hi))| m.get(k).map(|v| v >= lo && v <= hi)).collect();
    hits.iter().filter(|h| **h).count() as f32 / hits.len().max(1) as f32
}

/// A profile from the references, checked leave-one-out against `other` (unrelated work's measures)
pub fn profile(name: &str, refs: Vec<(String, Measures)>, other: &[Measures]) -> Profile {
    let all: Vec<&Measures> = refs.iter().map(|r| &r.1).collect();
    let spans = ranges(&all);
    let (mut own, mut oth) = (Vec::new(), Vec::new());
    if refs.len() >= 3 {
        for i in 0..refs.len() {
            let rest: Vec<&Measures> = all.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, m)| *m).collect();
            let r = ranges(&rest);
            own.push(inside(all[i], &r));
            oth.extend(other.iter().map(|m| inside(m, &r)));
        }
    }
    let (inside_own, inside_other) = ((!own.is_empty()).then(|| mean(&own)), (!oth.is_empty()).then(|| mean(&oth)));
    let tells = inside_own.zip(inside_other).map(|(a, b)| a > b);
    Profile { name: name.into(), refs, ranges: spans, inside_own, inside_other, tells }
}

/// One measure out of range: by how much, which way to move, and the window where it's furthest out
#[derive(Clone, Debug, Serialize)]
pub struct Out {
    pub measure: String,
    pub value: f32,
    pub range: (f32, f32),
    /// "raise" or "lower"
    pub move_: String,
    pub at: Option<(f64, f64)>,
}

/// The piece against the profile: the whole piece's measures, and its windows' for where
pub fn critique(p: &Profile, whole: &Measures, windows: &[((f64, f64), Measures)]) -> Vec<Out> {
    let mut out = Vec::new();
    for (k, (lo, hi)) in &p.ranges {
        let Some(&v) = whole.get(k) else { continue };
        if v >= *lo && v <= *hi {
            continue;
        }
        let low = v < *lo;
        let far = |m: &Measures| m.get(k).map_or(0.0, |x| if low { lo - x } else { x - hi });
        let at = windows.iter().max_by(|a, b| far(&a.1).total_cmp(&far(&b.1))).filter(|w| far(&w.1) > 0.0).map(|w| w.0);
        out.push(Out { measure: k.clone(), value: v, range: (*lo, *hi), move_: if low { "raise".into() } else { "lower".into() }, at });
    }
    out
}
