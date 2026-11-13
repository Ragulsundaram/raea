//! Geometry axes are checks, not heads (docs/fingerprint.feature FP13c): from a shot's things (things.rs), its depth
//! and its camera's path, four measures that `raea check --things` reports as findings, each with red-team cases
//! below, and none ever trained as a head.
//!
//! - floating: the ground right under a thing is farther than the thing's own base, for a second or more, while the
//!   thing's base is in the frame
//! - passing through: two things overlap and swap which is nearer while they still overlap, so one went through the other
//! - a camera that changes speed mid-move: inside a move, away from its first and last half second, the camera's
//!   speed doubles or halves from one half second to the next
//! - a frame with nothing on it: no detail, away from the piece's first and last quarter second (its depth is the
//!   model's guess at a blank picture, so it isn't asked)

use serde::{Deserialize, Serialize};

/// The ground under a base this much farther (in inverse depth, relative) is a gap
const GAP: f32 = 0.15;
/// A gap this long is floating, seconds
const FLOATS: f64 = 1.0;
/// Two boxes overlapping by this much of the smaller are touching
const TOUCH: f32 = 0.2;
/// Two depths this far apart (relative) are an order; nearer, they're level
const ORDER: f32 = 0.05;
/// The camera moving at this much of the frame a second or more is a move
const MOVING: f32 = 0.01;
/// The ease at each end of a move a speed change is allowed in, seconds
const EASE: f64 = 0.5;
/// A speed this many times the half second before's (or a fraction of it) is a change
const JUMP: f32 = 2.0;
/// A frame's grey spread under this has nothing on it
const BLANK: f32 = 0.02;
/// A blank frame this near the piece's start or end is a fade, seconds
const FADE: f64 = 0.25;

/// A thing at a moment: its box (x0,y0,x1,y1 in 0-1), its base's inverse depth (bigger is nearer) and the ground's
/// right under its box, None when its box reaches the frame's foot
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Seen {
    pub t: f64,
    pub b: [f32; 4],
    pub near: f32,
    pub base: f32,
    pub under: Option<f32>,
}

/// The camera between two frames: its pan in frames a second and its zoom a second (0 is none, above 0 in)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Move {
    pub t: f64,
    pub dx: f32,
    pub dy: f32,
    pub zoom: f32,
}

/// A frame's detail: its grey's spread and its depth's, 0-1
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    pub t: f64,
    pub detail: f32,
    pub depth: Option<f32>,
}

/// A finding for `raea check` (its Issue: a rule, a rank, a time, the words)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Issue {
    pub rule: String,
    pub p: u8,
    pub at: f64,
    pub to: f64,
    pub says: String,
}

fn issue(rule: &str, p: u8, at: f64, to: f64, says: String) -> Issue {
    Issue { rule: rule.into(), p, at, to, says }
}

/// Each run as an issue of `rule`, in its words
fn each(rule: &str, p: u8, found: Vec<(f64, f64)>, says: impl Fn(f64, f64) -> String) -> Vec<Issue> {
    let mut out = Vec::new();
    for (a, b) in found {
        out.push(issue(rule, p, a, b, says(a, b)));
    }
    out
}

/// Runs where `bad` holds of consecutive seens, at least `long` seconds, as (first, last)
fn runs<T>(xs: &[T], t: impl Fn(&T) -> f64, bad: impl Fn(&T) -> bool, long: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut from: Option<f64> = None;
    let mut last = 0.0;
    for x in xs {
        if bad(x) {
            from.get_or_insert(t(x));
            last = t(x);
        } else if let Some(f) = from.take() {
            if last - f >= long {
                out.push((f, last));
            }
        }
    }
    if let Some(f) = from.filter(|f| last - f >= long) {
        out.push((f, last));
    }
    out
}

/// A thing whose ground is farther than its base for FLOATS or longer
pub fn floating(name: &str, seen: &[Seen]) -> Vec<Issue> {
    let gap = |s: &Seen| s.under.is_some_and(|u| u < s.base * (1.0 - GAP));
    each("floating", 2, runs(seen, |s| s.t, gap, FLOATS), |a, b| format!("{name} floats: there's a gap under it from {a:.1} s to {b:.1} s"))
}

fn overlap(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let area = |r: &[f32; 4]| ((r[2] - r[0]) * (r[3] - r[1])).max(1e-6);
    w * h / area(a).min(area(b))
}

/// Two things that swap which is nearer while they overlap: the order before and after a stretch of touching
pub fn passing(a: (&str, &[Seen]), b: (&str, &[Seen])) -> Vec<Issue> {
    let mut out = Vec::new();
    // which is nearer while touching: +1 a, -1 b, 0 level; reset when they part
    let mut was: Option<(i8, f64)> = None;
    for x in a.1 {
        let Some(y) = b.1.iter().find(|y| (y.t - x.t).abs() < 1e-3) else { continue };
        if overlap(&x.b, &y.b) < TOUCH {
            was = None;
            continue;
        }
        let d = (x.near - y.near) / x.near.max(y.near).max(1e-6);
        let now = if d > ORDER { 1 } else if d < -ORDER { -1 } else { 0 };
        match was {
            Some((o, t)) if o != 0 && now == -o => {
                out.push(issue("passing-through", 1, t, x.t, format!("{} and {} pass through each other between {t:.1} s and {:.1} s", a.0, b.0, x.t)));
                was = Some((now, x.t));
            }
            _ if now != 0 => was = Some((now, x.t)),
            _ => {}
        }
    }
    out
}

/// The camera's speed, frames a second (a zoom of 10% a second counts as a tenth of a frame)
fn speed(m: &Move) -> f32 {
    (m.dx * m.dx + m.dy * m.dy).sqrt() + m.zoom.abs()
}

/// Mean speed of the moves in [a, b)
fn mean_speed(ms: &[Move], a: f64, b: f64) -> Option<f32> {
    let (mut n, mut sum) = (0usize, 0.0f32);
    for m in ms {
        if m.t >= a && m.t < b {
            n += 1;
            sum += speed(m);
        }
    }
    (n > 0).then(|| sum / n as f32)
}

/// Inside each move of one shot (a run moving at MOVING or more), away from its eases, a half second whose speed is
/// JUMP times the one before's, or that fraction of it
pub fn speed_change(ms: &[Move]) -> Vec<Issue> {
    let mut out = Vec::new();
    for (a, b) in runs(ms, |m| m.t, |m| speed(m) >= MOVING, 2.0 * EASE + 1.0) {
        let mut t = a + EASE;
        while t + 1.0 <= b - EASE + 1e-6 {
            if let (Some(s0), Some(s1)) = (mean_speed(ms, t, t + 0.5), mean_speed(ms, t + 0.5, t + 1.0)) {
                let r = s1 / s0.max(1e-6);
                if r >= JUMP || r <= 1.0 / JUMP {
                    out.push(issue("camera-speed", 2, t + 0.5, t + 1.0, format!("the camera {} mid-move at {:.1} s", if r > 1.0 { "lurches faster" } else { "suddenly slows" }, t + 0.5)));
                    t += 0.5;
                }
            }
            t += 0.5;
        }
    }
    out
}

/// Frames with nothing on them, not at the piece's very start or end (`dur`)
pub fn empty(frames: &[Frame], dur: f64) -> Vec<Issue> {
    let blank = |f: &Frame| f.detail < BLANK && f.t > FADE && f.t < dur - FADE;
    each("empty-frame", 1, runs(frames, |f| f.t, blank, 0.0), |a, b| format!("nothing is on screen from {a:.1} s to {b:.1} s"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(t: f64, b: [f32; 4], near: f32, base: f32, under: Option<f32>) -> Seen {
        Seen { t, b, near, base, under }
    }

    #[test]
    fn floating_is_a_lasting_gap_under_a_base_in_frame() {
        let on_ground: Vec<Seen> = (0..12).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.6, 0.8], 0.5, 0.6, Some(0.59))).collect();
        assert_eq!(floating("cup", &on_ground), [], "resting: the ground under it is as near as its base");
        let cropped: Vec<Seen> = (0..12).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.6, 1.0], 0.5, 0.6, None)).collect();
        assert_eq!(floating("cup", &cropped), [], "its base is out of frame");
        let blip: Vec<Seen> = (0..12).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.6, 0.8], 0.5, 0.6, Some(if i == 5 { 0.2 } else { 0.6 }))).collect();
        assert_eq!(floating("cup", &blip), [], "one bad depth isn't a gap");
        let floats: Vec<Seen> = (0..12).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.6, 0.8], 0.5, 0.6, Some(0.3))).collect();
        assert_eq!(floating("cup", &floats).len(), 1);
    }

    #[test]
    fn passing_through_is_a_swap_while_touching() {
        let a: Vec<Seen> = (0..8).map(|i| at(i as f64 * 0.25, [0.2 + i as f32 * 0.05, 0.3, 0.5 + i as f32 * 0.05, 0.8], 0.8, 0.8, None)).collect();
        let b: Vec<Seen> = (0..8).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.7, 0.8], 0.4, 0.4, None)).collect();
        assert_eq!(passing(("a", &a), ("b", &b)), [], "a walks in front of b: always nearer");
        let through: Vec<Seen> = (0..8).map(|i| at(i as f64 * 0.25, [0.4, 0.3, 0.7, 0.8], if i < 4 { 0.6 } else { 0.2 }, 0.4, None)).collect();
        assert_eq!(passing(("a", &through), ("b", &b)).len(), 1, "a goes from in front of b to behind it while overlapping");
        let parted: Vec<Seen> = (0..8).map(|i| at(i as f64 * 0.25, if (3..5).contains(&i) { [0.0, 0.0, 0.1, 0.1] } else { [0.4, 0.3, 0.7, 0.8] }, if i < 4 { 0.6 } else { 0.2 }, 0.4, None)).collect();
        assert_eq!(passing(("a", &parted), ("b", &b)), [], "they part, then come back the other way round");
    }

    fn pan(t: f64, dx: f32) -> Move {
        Move { t, dx, dy: 0.0, zoom: 0.0 }
    }

    #[test]
    fn a_speed_change_is_inside_a_move_not_its_ease() {
        let steady: Vec<Move> = (0..32).map(|i| pan(i as f64 / 8.0, 0.2)).collect();
        assert_eq!(speed_change(&steady), []);
        // eases in and out over a second each end: half-second steps never double past the ease
        let eased: Vec<Move> = (0..48).map(|i| {
            let t = i as f64 / 8.0;
            pan(t, 0.2 * (t.min(6.0 - t).clamp(0.0, 1.0) as f32).max(0.15))
        }).collect();
        assert_eq!(speed_change(&eased), [], "a gentle ease isn't a lurch");
        let lurch: Vec<Move> = (0..32).map(|i| pan(i as f64 / 8.0, if i >= 16 { 0.6 } else { 0.2 })).collect();
        let c = speed_change(&lurch);
        assert_eq!(c.len(), 1, "{c:?}");
        assert!((c[0].at - 2.0).abs() < 0.01);
        let still_then_pan: Vec<Move> = (0..32).map(|i| pan(i as f64 / 8.0, if i >= 16 { 0.3 } else { 0.0 })).collect();
        assert_eq!(speed_change(&still_then_pan), [], "starting to move isn't a change mid-move");
    }

    #[test]
    fn an_empty_frame_is_mid_piece_with_no_detail() {
        let f = |t: f64, detail: f32| Frame { t, detail, depth: Some(detail) };
        let fade: Vec<Frame> = [f(0.0, 0.0), f(0.5, 0.3), f(1.0, 0.3), f(1.9, 0.0)].into();
        assert_eq!(empty(&fade, 2.0), [], "fading in and out at the ends");
        let dark = [f(0.5, 0.03), f(1.0, 0.04)];
        assert_eq!(empty(&dark, 2.0), [], "a dark frame with something in it");
        let black = [f(0.5, 0.3), f(1.0, 0.001), f(1.25, 0.002), f(1.5, 0.3)];
        assert_eq!(empty(&black, 2.0).len(), 1);
    }
}
