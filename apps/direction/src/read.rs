//! `text-read`, raea's reading check (docs/direction.feature DR28). Each text on screen is given three times: the
//! frames it is in the world (`on`), the frames it can be read on (`visible`: in the frame, at an opacity, size and
//! contrast against what is behind it that can be read, and covered by no other layer past a share), and the frames
//! OCR reads its words on (`read`, the render's Ocr). Its reading time is held against `visible`, not `on`: a text
//! faded in over half its time, or behind a part for some of it, has less time to be read than it is up, and the
//! finding says which frames were lost and why. A text that can be read where OCR doesn't read it is broken. raea
//! check runs it on every composition's newest render with no rule asking for it, as raea's own, beside the floor.

use std::collections::BTreeMap;

use raea_core_facts::{Contrast, Covers, Facts, Finding, Layer, Level, Line, Ocr, Rule, Text, word_key as norm};

use crate::systems::{ident, quote};

/// Frames OCR may miss inside what it reads before that's a fault (a word dropped for a frame or two, as textread's
/// GAP allows).
const GAP: usize = 3;

/// What the check holds a text to, each a value its rule may change: seconds a word (`word`) and at least (`least`),
/// as the floor's reading rule; the least opacity, letter height (px at 1080 on the short side) and contrast that can
/// be read; the most of it another layer may cover; the share of its words OCR must read.
struct Limits {
    word: f64,
    least: f64,
    opacity: f64,
    size: f64,
    contrast: f64,
    covered: f64,
    share: f64,
}

impl Limits {
    fn of(rule: &Rule) -> Limits {
        let v = |n: &str, d: f64| rule.number(n).unwrap_or(d);
        Limits { word: v("word", 0.3), least: v("least", 1.0), opacity: v("opacity", 0.5), size: v("size", 24.0), contrast: v("contrast", 3.0), covered: v("covered", 0.5), share: v("share", 0.5) }
    }
}

/// One frame a text is on: why it can't be read there (None: it can), whether OCR read it (None: OCR didn't run),
/// and what OCR read.
struct Seen {
    frame: i64,
    lost: Option<String>,
    read: Option<bool>,
    ocr: String,
}

/// A text across the frames: its words, its layer key, the largest it's set, and each frame it's on.
struct Track {
    words: String,
    key: Option<String>,
    size: f64,
    seen: Vec<Seen>,
}

/// An entity's opacity: its own layer's, else its nearest parent's.
fn opacity(w: &Facts, mut e: u32) -> f64 {
    loop {
        if let Some(l) = w.get::<Layer>(e) {
            return l.opacity;
        }
        match w.entities.get(e as usize).and_then(|x| x.parent) {
            Some(p) => e = p,
            None => return 1.0,
        }
    }
}

/// Why a text can't be read on this frame (`covered`: the most of it one layer covers), or None.
fn lost(w: &Facts, e: u32, t: &Text, l: &Limits, covered: f64) -> Option<String> {
    if let Some(b) = t.box_ {
        let inside = |a: f64, z: f64| (z.min(1.0) - a.max(0.0)).max(0.0);
        let area = ((b[2] - b[0]) * (b[3] - b[1])).max(1e-9);
        if inside(b[0], b[2]) * inside(b[1], b[3]) / area < 0.9 {
            return Some("partly off the frame".into());
        }
    }
    let o = opacity(w, e);
    if o < l.opacity {
        return Some(format!("at {:.0}% opacity", o * 100.0));
    }
    if let Some(s) = t.size.filter(|s| *s < l.size) {
        return Some(format!("{s:.0} px high"));
    }
    // covered before contrast: what covers it is what its pixels then measure against
    if covered > l.covered {
        return Some(format!("{:.0}% covered", covered * 100.0));
    }
    w.get::<Contrast>(e).filter(|c| c.ratio < l.contrast).map(|c| format!("{:.1}:1 against what's behind it", c.ratio))
}

/// The words said or sung over the piece, each with its start in seconds.
fn said(worlds: &[Facts]) -> Vec<(String, f64)> {
    let Some(w) = worlds.iter().find(|w| !w.find(&["Line"]).is_empty()) else { return Vec::new() };
    w.find(&["Line"]).iter().filter_map(|e| w.get::<Line>(e.id)).flat_map(|l| l.words).map(|x| (norm(&x.text), x.start)).collect()
}

/// Runs of frames one after another in `frames` (sorted), each [first, the frame after its last).
fn runs(frames: impl IntoIterator<Item = i64>) -> Vec<[i64; 2]> {
    let mut out: Vec<[i64; 2]> = Vec::new();
    for f in frames {
        match out.last_mut() {
            Some(r) if r[1] == f => r[1] = f + 1,
            _ => out.push([f, f + 1]),
        }
    }
    out
}

/// Runs closer than `gap` frames to the one before, joined.
fn joined(runs: Vec<[i64; 2]>, gap: i64) -> Vec<[i64; 2]> {
    let mut out: Vec<[i64; 2]> = Vec::new();
    for r in runs {
        match out.last_mut() {
            Some(p) if r[0] - p[1] < gap => p[1] = r[1],
            _ => out.push(r),
        }
    }
    out
}

/// A number of frames as a person reads it: "2 frames" under a tenth of a second, else seconds.
fn span(frames: i64, fps: f64) -> String {
    match frames {
        1 => "1 frame".into(),
        n if (n as f64) < 0.1 * fps => format!("{n} frames"),
        n => format!("{:.1} s", n as f64 / fps),
    }
}

/// "frames 10-22 at 20% opacity; frames 30-41 60% covered": the frames of [from, to) lost, grouped by why (three
/// groups at most); a frame it isn't drawn on is "not shown".
fn why_lost(from: i64, to: i64, seen: &[&Seen]) -> String {
    let mut groups: Vec<([i64; 2], &str)> = Vec::new();
    for f in from..to {
        let why = match seen.iter().find(|s| s.frame == f) {
            Some(s) => match s.lost.as_deref() {
                Some(w) => w,
                None => continue,
            },
            None => "not shown",
        };
        match groups.last_mut() {
            Some((r, w)) if r[1] == f && *w == why => r[1] = f + 1,
            _ => groups.push(([f, f + 1], why)),
        }
    }
    let shown: Vec<String> = groups.iter().take(3).map(|([a, b], w)| if b - a == 1 { format!("frame {a} {w}") } else { format!("frames {a}-{} {w}", b - 1) }).collect();
    let more = groups.len().saturating_sub(3);
    format!("{}{}", shown.join("; "), if more > 0 { format!("; and {more} more") } else { String::new() })
}

/// The check's findings over `worlds` (every frame of the composition, in order, with its render joined).
pub fn read(rule: &Rule, worlds: &[Facts]) -> Vec<Finding> {
    let l = Limits::of(rule);
    let Some(fps) = worlds.first().map(|w| w.fps) else { return Vec::new() };
    let lvl = |d: Level| rule.level.unwrap_or(d);
    let mut tracks: BTreeMap<String, Track> = BTreeMap::new();
    // what wasn't measured, and what it waits for
    let mut waits: BTreeMap<String, String> = BTreeMap::new();
    for w in worlds {
        for c in ["Ocr", "Covers"] {
            if let Some(why) = w.resources.get("Waits").and_then(|v| v.get(c)).and_then(|v| v.as_str()) {
                waits.entry(c.to_string()).or_insert_with(|| why.to_string());
            }
        }
        // the most of each layer one other layer covers
        let mut covered: BTreeMap<u32, f64> = BTreeMap::new();
        for e in w.find(&["Covers"]) {
            for c in w.get::<Covers>(e.id).map(|c| c.0).unwrap_or_default() {
                let s = covered.entry(c.layer).or_default();
                *s = s.max(c.share);
            }
        }
        for e in w.find(&["Text"]) {
            let Some(t) = w.get::<Text>(e.id) else { continue };
            let (id, key) = ident(w, e.id, &t);
            let lost = lost(w, e.id, &t, &l, covered.get(&e.id).copied().unwrap_or(0.0));
            let ocr = w.get::<Ocr>(e.id);
            let tr = tracks.entry(id).or_insert_with(|| Track { words: t.words.clone(), key, size: 0.0, seen: Vec::new() });
            // a text laid out with no size known is held as a caption
            tr.size = tr.size.max(t.size.unwrap_or(f64::INFINITY));
            tr.seen.push(Seen { frame: w.frame, lost, read: ocr.as_ref().map(|o| o.share >= l.share), ocr: ocr.map(|o| o.words).unwrap_or_default() });
        }
    }
    let said = said(worlds);
    let mut out = Vec::new();
    // a text that leaves and comes back within half a second is one text that blinks, not two
    let blink = (0.5 * fps).round() as i64;
    for tr in tracks.values().filter(|t| t.size >= l.size) {
        for [from, to] in joined(runs(tr.seen.iter().map(|s| s.frame)), blink) {
            let seen: Vec<&Seen> = tr.seen.iter().filter(|s| (from..to).contains(&s.frame)).collect();
            let at = |f: Finding| f.about(tr.words.clone(), tr.key.clone());
            let q = quote(&tr.words);
            // the time it needs to be read, against the time it can be: a caption the voice says as it shows needs none
            let n = tr.words.split_whitespace().count();
            let need = (l.word * n as f64).max(l.least);
            let can_frames = seen.iter().filter(|s| s.lost.is_none()).count() as i64;
            let can = can_frames as f64 / fps;
            let (a, b) = (from as f64 / fps, to as f64 / fps);
            let tokens: Vec<String> = tr.words.split_whitespace().map(norm).filter(|t| !t.is_empty()).collect();
            let spoken = !tokens.is_empty() && tokens.iter().all(|t| said.iter().any(|(w, s)| w == t && *s >= a - 0.3 && *s <= b + 0.1));
            if can + 0.05 < need && !spoken {
                let says = if can_frames == 0 {
                    format!("{q} is up {} but can't be read on any frame of it ({}); {n} words need {need:.1} s", span(to - from, fps), why_lost(from, to, &seen))
                } else if can_frames < to - from {
                    format!("{q} is up {} but can be read for {} ({}); {n} words need {need:.1} s", span(to - from, fps), span(can_frames, fps), why_lost(from, to, &seen))
                } else {
                    format!("{q} is on for {}; {n} words need {need:.1} s", span(to - from, fps))
                };
                out.push(at(Finding::new(&rule.id, lvl(Level::Warn), says, &rule.by)).frames(from, to).measure("readable", (can * 10.0).round() / 10.0, "s", Some((need * 10.0).round() / 10.0)));
            }
            // what can be read and OCR doesn't read is broken; a word with fewer than two letters or digits isn't held
            if norm(&tr.words).chars().count() < 2 {
                continue;
            }
            let judged: Vec<&&Seen> = seen.iter().filter(|s| s.lost.is_none() && s.read.is_some()).collect();
            let unread: Vec<[i64; 2]> = runs(judged.iter().filter(|s| s.read == Some(false)).map(|s| s.frame)).into_iter().filter(|[a, b]| (b - a) as usize > GAP).collect();
            for [ua, ub] in &unread {
                let got = judged.iter().find(|s| s.frame == *ua).map_or("", |s| s.ocr.as_str());
                let heard = if got.trim().is_empty() { "OCR reads nothing in its box".to_string() } else { format!("OCR reads {} in its box", quote(got)) };
                let says = format!("{q} can be read on frames {ua}-{} and OCR doesn't read it there ({heard}): it's broken", ub - 1);
                out.push(at(Finding::new(&rule.id, lvl(Level::Error), says, &rule.by)).frames(*ua, *ub).fix("set it so it reads: a face, weight, size or treatment OCR reads, and nothing drawn through it"));
            }
            let read = judged.iter().filter(|s| s.read == Some(true)).count();
            if unread.is_empty() && !judged.is_empty() && read * 2 < judged.len() {
                let says = format!("{q} is read by OCR on {read} of the {} frames it can be read on", judged.len());
                out.push(at(Finding::new(&rule.id, lvl(Level::Warn), says, &rule.by)).frames(from, to).measure("read", read as f64, "frames", Some(judged.len() as f64 / 2.0)));
            }
        }
    }
    // what it didn't see, said
    if let Some(why) = waits.get("Ocr") {
        out.push(Finding::new(&rule.id, Level::Note, format!("no text was held to what OCR reads: OCR waits for {why}"), &rule.by));
    }
    if let Some(why) = waits.get("Covers") {
        out.push(Finding::new(&rule.id, Level::Note, format!("what covers each text wasn't counted: it waits for {why}"), &rule.by));
    }
    out
}

#[cfg(test)]
mod tests;
