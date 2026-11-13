//! Scoring a piece (FP3a): every axis per 2 s window every 1 s, from the file's columns. An axis reads its learned
//! head once that beats its word head on held-out evidence, else its words (always advice). A window is flagged on an
//! axis above the score that flags 10% of clean held-out work; a gating flag is a gating head's flag in its strong
//! band (precision 0.9 held out). Flagged windows in a row make a moment, the thing a note is expected on.

use raea_sys_fingerprint::evidence::{features, slice};
use raea_sys_fingerprint::taxonomy::Fingerprint;
use raea_sys_fingerprint::train::{AxisHead, Trained, scores};
use raea_sys_fingerprint::words::Words;
use raea_sys_columns::Columns;
use raea_sys_columns::window::Modality;
use serde::Serialize;

pub const WIN: f64 = 2.0;
pub const HOP: f64 = 1.0;
/// A word head's flag before any clean evidence has set one: a z this far over the vocabulary
pub const WORD_FLAG: f32 = 2.5;

#[derive(Clone, Debug, Serialize)]
pub struct AxisScore {
    pub axis: String,
    pub value: f32,
    /// "head" or "words"
    pub by: &'static str,
    /// gating, advice or words
    pub state: String,
    /// strong, plausible or weak, for a learned head
    #[serde(skip_serializing_if = "Option::is_none")]
    pub band: Option<&'static str>,
    pub flagged: bool,
    pub gating: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Window {
    pub a: f64,
    pub b: f64,
    pub axes: Vec<AxisScore>,
    /// the nearest of the vocabulary's descriptions: what is seen and heard
    pub seen: Option<&'static str>,
    pub heard: Option<&'static str>,
    /// the sound's mean level, dBFS
    pub level: f32,
    /// the person's weighting over the axes, once they've taught enough
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall: Option<f32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Moment {
    pub a: f64,
    pub b: f64,
    pub axis: String,
    /// the axis in the person's words (its first)
    pub words: String,
    pub by: &'static str,
    pub state: String,
    pub band: Option<&'static str>,
    pub gating: bool,
    pub peak: f32,
    /// where in the frame, once the axis has a patch head that points (FP12a)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<Where>,
}

/// Where a moment's defect is: its box in the frame at time `t` (fractions, [x0, y0, x1, y1]) and the render's layers
/// under it, most of the box first
#[derive(Clone, Debug, Serialize)]
pub struct Where {
    pub t: f64,
    pub region: [f64; 4],
    pub layers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Scored {
    pub file: String,
    pub media: String,
    pub dur: f64,
    pub fingerprint: String,
    pub moments: Vec<Moment>,
    pub windows: Vec<Window>,
}

fn rank(b: Option<&str>) -> u8 {
    match b {
        Some("strong") => 3,
        Some("plausible") => 2,
        Some(_) => 1,
        None => 0,
    }
}

/// One axis on one window's features
fn one(h: Option<&AxisHead>, words: Option<&Words>, axis: &str, m: Modality, row: &[f32]) -> Option<AxisScore> {
    let learned = h.and_then(|h| h.head.as_ref().map(|f| (h, f))).filter(|(h, _)| match (h.held_out, h.words) {
        (Some(a), Some(w)) => a >= w,
        (None, Some(_)) => false,
        _ => true,
    });
    if let Some((h, f)) = learned {
        let v = f.score(&row[slice(m)]);
        let band = if h.strong.is_some_and(|t| v >= t) {
            "strong"
        } else if h.plausible.is_some_and(|t| v >= t) {
            "plausible"
        } else {
            "weak"
        };
        let flagged = h.flag.is_some_and(|t| v > t);
        let gating = flagged && h.state == "gating" && band == "strong";
        return Some(AxisScore { axis: axis.into(), value: v, by: "head", state: h.state.clone(), band: Some(band), flagged, gating });
    }
    let z = words?.z(axis, row)?;
    let flagged = z > h.and_then(|h| h.word_flag).unwrap_or(WORD_FLAG);
    Some(AxisScore { axis: axis.into(), value: z, by: "words", state: "words".into(), band: None, flagged, gating: false })
}

pub fn score(fp: &Fingerprint, words: Option<&Words>, trained: Option<&Trained>, c: &Columns, file: &str) -> Scored {
    let mut windows = Vec::new();
    let mut a = 0.0;
    loop {
        let b = (a + WIN).min(c.dur.max(0.25));
        let row = features(c, a, b);
        let nearest = |m: Modality| words.filter(|_| m.in_file(c)).and_then(|w| w.nearest(&row, m));
        let head = |name: &str| trained.and_then(|t| t.heads.iter().find(|h| h.axis == name));
        // an axis judges only what the file has: a script's windows hold no picture to call blurry
        let axes: Vec<AxisScore> = fp.axes.iter().filter(|ax| ax.modality.in_file(c)).filter_map(|ax| one(head(&ax.name), words, &ax.name, ax.modality, &row)).collect();
        let (l0, l1) = ((a * 10.0) as usize, ((b * 10.0) as usize).max((a * 10.0) as usize + 1));
        let lv = c.level.get(l0.min(c.level.len())..l1.min(c.level.len())).unwrap_or(&[]);
        let level = if lv.is_empty() { -100.0 } else { lv.iter().sum::<f32>() / lv.len() as f32 };
        let overall = trained.and_then(|t| t.weighting.as_ref().map(|w| w.score(&scores(&t.heads, &row))));
        windows.push(Window {
            a,
            b,
            axes,
            seen: nearest(Modality::Picture),
            heard: nearest(Modality::Sound),
            level,
            overall,
        });
        if a + HOP + WIN > c.dur + 1e-6 {
            break;
        }
        a += HOP;
    }
    let mut moments: Vec<Moment> = Vec::new();
    for ax in &fp.axes {
        let mut open: Option<Moment> = None;
        for w in &windows {
            match (w.axes.iter().find(|s| s.axis == ax.name && s.flagged), open.as_mut()) {
                (Some(s), Some(m)) if w.a <= m.b => {
                    m.b = w.b;
                    m.peak = m.peak.max(s.value);
                    m.gating |= s.gating;
                    if rank(s.band) > rank(m.band) {
                        m.band = s.band;
                    }
                }
                (Some(s), _) => {
                    moments.extend(open.take());
                    let words = ax.words.first().cloned().unwrap_or_default();
                    open = Some(Moment { a: w.a, b: w.b, axis: ax.name.clone(), words, by: s.by, state: s.state.clone(), band: s.band, gating: s.gating, peak: s.value, at: None });
                }
                (None, _) => {}
            }
        }
        moments.extend(open);
    }
    moments.sort_by(|x, y| (y.gating, rank(y.band)).cmp(&(x.gating, rank(x.band))).then(x.a.total_cmp(&y.a)));
    Scored { file: file.into(), media: c.media.clone(), dur: c.dur, fingerprint: fp.name.clone(), moments, windows }
}

/// m:ss
pub fn clock(t: f64) -> String {
    let s = t.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn label(m: &Moment) -> String {
    match (m.gating, m.band) {
        (true, _) => "gating".into(),
        (false, Some(b)) => format!("{b}, {}", m.state),
        (false, None) => "words, advice".into(),
    }
}

/// The summary first: the moments the fingerprint expects a note on, gating ones first
pub fn summary(s: &Scored) -> String {
    let gating = s.moments.iter().filter(|m| m.gating).count();
    let mut out = format!(
        "{} ({}), fingerprint {}: {} moment{} to look at, {gating} gating\n",
        s.file,
        clock(s.dur),
        s.fingerprint,
        s.moments.len(),
        if s.moments.len() == 1 { "" } else { "s" }
    );
    for m in &s.moments {
        let at = m.at.as_ref().map_or(String::new(), |w| {
            let on = if w.layers.is_empty() { String::new() } else { format!(" on {}", w.layers.join(", ")) };
            format!(", {}{on}", crate::patch::place(w.region))
        });
        out += &format!("  {}-{}  {}: \"{}\" ({}){at}\n", clock(m.a), clock(m.b), m.axis, m.words, label(m));
    }
    if s.moments.is_empty() {
        out += "  nothing flagged\n";
    }
    out
}

/// Second by second in words: what is seen, what is heard, which axes flag (earshot's timeline)
pub fn timeline(s: &Scored) -> String {
    let mut out = String::new();
    for w in &s.windows {
        let flags: Vec<String> = w.axes.iter().filter(|x| x.flagged).map(|x| format!("{} ({})", x.axis, if x.gating { "gating" } else { x.band.unwrap_or("words") })).collect();
        out += &format!(
            "{}-{}  seen: {} · heard: {} · {:.0} dB{}\n",
            clock(w.a),
            clock(w.b),
            w.seen.unwrap_or("?"),
            w.heard.unwrap_or("?"),
            w.level,
            if flags.is_empty() { String::new() } else { format!(" · flags: {}", flags.join(", ")) }
        );
    }
    out
}
