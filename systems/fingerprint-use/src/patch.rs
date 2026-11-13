//! Patch heads (docs/fingerprint.feature FP12a): a picture axis's head on SigLIP 2's 14 x 14 patch tokens, so a flag
//! says where in the frame, not only whether. Trained from region labels: a defect frame's patches inside its box
//! are positive, its patches outside and every other frame's negative. Held out by source, a head points when its top
//! patch falls in the box; it says where only at POINTS or better from FEW frames, as FP0f measured (0.85-0.96).

use raea_sys_columns::SIG;
use raea_sys_fingerprint::head::{Logistic, Scaler};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const GRID: usize = 14;
pub const CELLS: usize = GRID * GRID;
/// Held out by source, how often the top patch must fall in the box for a head to say where (FP12a)
pub const POINTS: f64 = 0.85;
/// The defect frames a held-out fold trains on: FP0f's 8 marked frames
pub const FEW: usize = 8;
/// The head's C (as the axis heads')
const C: f64 = 1.0;
/// Where a fingerprint keeps its patch heads
pub const FILE: &str = "patches.json";

/// One frame's patch tokens, its source, and where its defect is (`axis` "*" for a clean frame)
#[derive(Clone, Debug)]
pub struct Frame {
    pub source: String,
    pub axis: String,
    /// the defect put in (damage.rs LOCAL), or "mark"
    pub kind: String,
    /// CELLS x SIG, row by row from the top left
    pub patches: Vec<f32>,
    pub inside: Vec<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchHead {
    pub axis: String,
    pub head: Logistic,
    /// held out by source from FEW frames: the share of the held-out source's frames whose top patch is in the box
    pub pointing: Option<f64>,
    /// the same for each kind of defect on this axis
    pub by_kind: BTreeMap<String, f64>,
    /// defect frames the head was trained on
    pub frames: usize,
}

impl PatchHead {
    /// Whether it may say where: pointing at POINTS or better, held out
    pub fn points(&self) -> bool {
        self.pointing.is_some_and(|p| p >= POINTS)
    }
}

fn fit(train: &[&Frame], axis: &str) -> Logistic {
    let (mut rows, mut y): (Vec<&[f32]>, Vec<bool>) = (Vec::new(), Vec::new());
    for f in train {
        for (i, p) in f.patches.chunks(SIG).enumerate() {
            rows.push(p);
            y.push(f.axis == axis && f.inside[i]);
        }
    }
    Logistic::fit(Scaler::fit(&rows), &rows, &y, None, C, true)
}

/// Each patch's score (log-odds), row by row from the top left
pub fn heat(h: &Logistic, patches: &[f32]) -> Vec<f32> {
    patches.chunks(SIG).map(|p| h.score(p)).collect()
}

fn top(heat: &[f32]) -> usize {
    (0..heat.len()).max_by(|a, b| heat[*a].total_cmp(&heat[*b])).unwrap_or(0)
}

fn pick<'a>(rng: &mut fastrand::Rng, v: &[&'a Frame], k: usize) -> Vec<&'a Frame> {
    let mut v = v.to_vec();
    rng.shuffle(&mut v);
    v.truncate(k);
    v
}

/// Held out by source: for each source in turn, a head trained on FEW of the axis's frames from the others (and as many
/// of their other frames, clean or another defect, times two) is tested on every frame of the axis from that source
pub fn pointing(frames: &[Frame], axis: &str, rng: &mut fastrand::Rng) -> (Option<f64>, BTreeMap<String, f64>) {
    let mut sources: Vec<&str> = frames.iter().filter(|f| f.axis == axis).map(|f| f.source.as_str()).collect();
    sources.sort();
    sources.dedup();
    if sources.len() < 2 {
        return (None, BTreeMap::new());
    }
    let mut kinds: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for s in &sources {
        let other: Vec<&Frame> = frames.iter().filter(|f| f.source != *s).collect();
        let on: Vec<&Frame> = other.iter().copied().filter(|f| f.axis == axis).collect();
        let off: Vec<&Frame> = other.iter().copied().filter(|f| f.axis != axis).collect();
        let train = [pick(rng, &on, FEW), pick(rng, &off, 2 * FEW)].concat();
        let h = fit(&train, axis);
        for f in frames.iter().filter(|f| f.source == *s && f.axis == axis) {
            let e = kinds.entry(f.kind.clone()).or_default();
            e.0 += usize::from(f.inside[top(&heat(&h, &f.patches))]);
            e.1 += 1;
        }
    }
    let rate = |(hit, n): (usize, usize)| hit as f64 / n.max(1) as f64;
    let all = kinds.values().fold((0, 0), |a, k| (a.0 + k.0, a.1 + k.1));
    (Some(rate(all)), kinds.into_iter().map(|(k, c)| (k, rate(c))).collect())
}

/// A patch head for each axis with defect frames: held out first, then trained on every frame
pub fn train(frames: &[Frame], seed: u64) -> Vec<PatchHead> {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut axes: Vec<&str> = frames.iter().map(|f| f.axis.as_str()).filter(|a| *a != "*").collect();
    axes.sort();
    axes.dedup();
    let all: Vec<&Frame> = frames.iter().collect();
    axes.into_iter()
        .map(|a| {
            let (pointing, by_kind) = pointing(frames, a, &mut rng);
            PatchHead { axis: a.into(), head: fit(&all, a), pointing, by_kind, frames: frames.iter().filter(|f| f.axis == a).count() }
        })
        .collect()
}

/// The box around the top patch and the patches joined to it that score within 1 of it (log-odds), as fractions of
/// the frame: [x0, y0, x1, y1]
pub fn region(heat: &[f32]) -> [f64; 4] {
    let t = top(heat);
    let (mut seen, mut todo) = (vec![false; heat.len()], vec![t]);
    let (mut x0, mut y0, mut x1, mut y1) = (GRID, GRID, 0, 0);
    while let Some(i) = todo.pop() {
        if seen[i] || heat[i] < heat[t] - 1.0 {
            continue;
        }
        seen[i] = true;
        let (x, y) = (i % GRID, i / GRID);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        if x > 0 { todo.push(i - 1) }
        if x + 1 < GRID { todo.push(i + 1) }
        if y > 0 { todo.push(i - GRID) }
        if y + 1 < GRID { todo.push(i + GRID) }
    }
    [x0, y0, x1, y1].map(|v| v as f64 / GRID as f64)
}

/// Where a box sits, in words: top left, middle, bottom right...
pub fn place(b: [f64; 4]) -> &'static str {
    let (x, y) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
    const NAMES: [[&str; 3]; 3] = [["top left", "top", "top right"], ["left", "middle", "right"], ["bottom left", "bottom", "bottom right"]];
    let third = |v: f64| (v * 3.0).min(2.0) as usize;
    NAMES[third(y)][third(x)]
}

pub fn save(dir: &Path, heads: &[PatchHead]) -> std::io::Result<()> {
    std::fs::write(dir.join(FILE), serde_json::to_vec(heads)?)
}

/// A fingerprint's patch heads; none before `raea fingerprint patches`
pub fn load(dir: &Path) -> Vec<PatchHead> {
    std::fs::read(dir.join(FILE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame whose patches carry `on` in their first value inside `b`'s cells, and seeded spread elsewhere
    fn frame(source: &str, axis: &str, b: Option<(usize, usize)>, seed: usize) -> Frame {
        let inside: Vec<bool> = (0..CELLS).map(|i| b.is_some_and(|(x, y)| (x..x + 4).contains(&(i % GRID)) && (y..y + 4).contains(&(i / GRID)))).collect();
        let patches = (0..CELLS * SIG).map(|j| if j % SIG == 0 && inside[j / SIG] { 1.0 } else { (((j * 31 + seed * 17) % 97) as f32 / 97.0 - 0.5) * 0.2 }).collect();
        Frame { source: source.into(), axis: axis.into(), kind: "blur_box".into(), patches, inside }
    }

    #[test]
    fn a_patch_head_points_at_its_box_held_out_by_source() {
        let mut v = Vec::new();
        for s in 0..3 {
            for k in 0..10 {
                v.push(frame(&format!("s{s}"), "blur", Some(((k * 3) % 10, (k * 7) % 10)), s * 100 + k));
                v.push(frame(&format!("s{s}"), "*", None, s * 100 + k + 50));
            }
        }
        let heads = train(&v, 1);
        assert_eq!(heads.len(), 1);
        assert!(heads[0].points(), "{:?}", heads[0].pointing);
        let f = &v[0];
        let r = region(&heat(&heads[0].head, &f.patches));
        assert_eq!(r, [0.0, 0.0, 4.0 / 14.0, 4.0 / 14.0]);
        assert_eq!(place(r), "top left");
    }

    #[test]
    fn one_source_is_not_held_out() {
        let v = vec![frame("s0", "blur", Some((2, 2)), 1), frame("s0", "*", None, 2)];
        assert_eq!(train(&v, 1)[0].pointing, None);
    }
}
