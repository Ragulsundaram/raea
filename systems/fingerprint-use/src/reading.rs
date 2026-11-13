//! A note's reading (docs/fingerprint.feature FP5a, FP5b): what a fresh agent session read a person's pinned note as
//! (a kind and an axis), the person's check of it, and where it points: its moment or span, its marks' region in the
//! frame (FP11b) and its box's pitch band on the sound (FP11c). Notes train only once the person has agreed with
//! enough readings; raea-app-fingerprint's notes.rs reads them, puts them to the person and learns from them.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// FP5b's bar, out of every 50 checked
pub const KIND_BAR: f64 = 45.0 / 50.0;
pub const AXIS_BAR: f64 = 40.0 / 50.0;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reading {
    pub note: String,
    pub text: String,
    pub who: String,
    pub media: PathBuf,
    pub t: f64,
    pub project: String,
    /// defect, direction, praise or both
    pub kind: String,
    pub axis: Option<String>,
    /// how it was read: kimi or claude
    pub by: String,
    /// the person's check: Some(true) agreed, Some(false) not; None until asked
    #[serde(default)]
    pub kind_ok: Option<bool>,
    #[serde(default)]
    pub axis_ok: Option<bool>,
    /// a span pin's end: the label covers t..to (FP11b)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<f64>,
    /// where in the frame the note's marks say the thing is, x0,y0,x1,y1 in 0-1 (FP11b)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<[f32; 4]>,
    /// the layers under the pin's x,y in the layer pass, on a render with one (FP5a)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<String>,
    /// a box on the sound's spectrogram: its pitch band, 0-1 of the mel scale (FP11c); its time is t..to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<[f32; 2]>,
}



/// The share of checked readings the person agreed with, for kind and for axis, and how many were checked
pub fn agreement(r: &[Reading]) -> (Option<f64>, Option<f64>, usize) {
    let share = |v: Vec<bool>| (!v.is_empty()).then(|| v.iter().filter(|x| **x).count() as f64 / v.len() as f64);
    let checked = r.iter().filter(|x| x.kind_ok.is_some()).count();
    (share(r.iter().filter_map(|x| x.kind_ok).collect()), share(r.iter().filter_map(|x| x.axis_ok).collect()), checked)
}

/// Whether the readings have earned training (FP5b): 50 checked, the kind right 45 times and the axis 40
pub fn earned(r: &[Reading]) -> bool {
    matches!(agreement(r), (Some(k), Some(a), n) if n >= 50 && k >= KIND_BAR && a >= AXIS_BAR)
}

/// A span pin's label: t..to inside the piece, widened about its middle to half a second when it's shorter (a box on a
/// click of sound)
pub fn span(t: f64, to: f64, dur: f64) -> (f64, f64) {
    let (a, b) = (t.max(0.0), to.min(dur));
    if b - a >= 0.5 {
        return (a, b);
    }
    let mid = ((a + b) / 2.0).clamp(0.25, (dur - 0.25).max(0.25));
    ((mid - 0.25).max(0.0), (mid + 0.25).min(dur))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_is_the_label_and_a_short_one_is_widened() {
        assert_eq!(span(2.0, 4.0, 10.0), (2.0, 4.0));
        let (a, b) = span(2.0, 2.1, 10.0);
        assert!((a - 1.8).abs() < 1e-9 && (b - 2.3).abs() < 1e-9, "a box on a click of sound is half a second about it: {a}..{b}");
        assert_eq!(span(9.9, 10.0, 10.0), (9.5, 10.0), "kept inside the piece");
    }

    #[test]
    fn notes_train_only_once_the_person_agrees_enough() {
        let r = |k: bool, a: bool| Reading { kind_ok: Some(k), axis_ok: Some(a), ..Default::default() };
        let mut v: Vec<Reading> = (0..45).map(|_| r(true, true)).collect();
        v.extend((0..5).map(|_| r(false, false)));
        assert!(earned(&v), "45 and 45 of 50");
        v[0] = r(false, true);
        assert!(!earned(&v), "44 kinds of 50 is under the bar");
        assert!(!earned(&v[..40]), "fewer than 50 checked");
    }
}
