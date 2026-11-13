//! A shot's things (docs/fingerprint.feature FP13a): per shot, its things, each with its box, its depth and the ground
//! under it at every half second, and one SigLIP 2 vector (its patch tokens pooled in its mask); the camera's path; and each frame's detail. A thing comes
//! from a person's mark, followed by EdgeTAM (grow.rs), or on a raea render from its layer pass (NAME.layers beside
//! the video), with no model run for its masks. Depth is the render's own when its AOV pass is beside it (NAME.aov,
//! `raea render EDIT --aov -o NAME.aov`), else Depth Anything 3 Small's (raea-perceive depth-values). The camera's path
//! is the render's own motion vectors from the same pass, else optical flow (raea-perceive flow); the shots are
//! raea-perceive cuts'. geometry.rs measures them.

use crate::geometry::{self, Frame, Issue, Move, Seen};
use raea_sys_fingerprint_use::grow::{self, Matte, bounds, perceive};
use anyhow::{Result, anyhow};
use raea_sys_columns::Encoders;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Samples a second: depth, things and detail
pub const FPS: f64 = 2.0;
/// Pairs a second the camera's path is read at
const FLOW_FPS: f64 = 8.0;
/// A layer covering less of the frame than this, or more (a background), isn't a thing
const SMALLEST: f32 = 0.002;
const LARGEST: f32 = 0.6;
/// The strip under a thing's box the ground is read in, of the frame's height
const STRIP: f32 = 0.05;
/// A box this near the frame's foot has its base out of frame
const FOOT: f32 = 0.97;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thing {
    pub name: String,
    /// "mark" (EdgeTAM from a person's mark) or "layer" (a render's layer pass)
    pub from: String,
    pub seen: Vec<Seen>,
    /// SigLIP 2 on the thing's box where it's largest
    pub embed: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shot {
    pub a: f64,
    pub b: f64,
    pub camera: Vec<Move>,
    pub things: Vec<Thing>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Things {
    pub file: String,
    pub dur: f64,
    pub shots: Vec<Shot>,
    pub frames: Vec<Frame>,
    /// geometry.rs's findings, for `raea check --things`
    pub issues: Vec<Issue>,
}

/// A mark to follow: its words, its moment, its label's span and its region
pub struct Mark {
    pub name: String,
    pub t: f64,
    pub a: f64,
    pub b: f64,
    pub region: [f32; 4],
}

/// A frame's 1/depth, w x h, each value over the frame's 99th percentile (1 is the nearest)
struct Depth {
    w: usize,
    h: usize,
    v: Vec<f32>,
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f32::total_cmp);
    Some(v[v.len() / 2])
}

fn quantile(v: &[f32], q: f64) -> f32 {
    raea_sys_fingerprint_use::profile::pct(v, q as f32)
}

/// A frame's 1/depth over its 99th percentile, clamped to 0-1
fn scaled(w: usize, h: usize, v: Vec<f32>) -> Depth {
    let top = quantile(&v, 0.99).max(1e-6);
    let v = v.into_iter().map(|x| (x / top).clamp(0.0, 1.0)).collect();
    Depth { w, h, v }
}

/// Every frame pair's camera move from raea-perceive flow, per second
fn camera(bin: &Path, file: &Path) -> Vec<Move> {
    // pyramidal Lucas-Kanade at 512: Vision's flow flattens a pan over plain ground to a quarter of it (11-12, a render)
    let Ok(o) = std::process::Command::new(bin).arg("flow").arg(file).args(["--fps", &FLOW_FPS.to_string(), "--side", "512", "--backend", "lk"]).output() else { return Vec::new() };
    let f = FLOW_FPS as f32;
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| {
            let g = |k: &str| v[k].as_f64().unwrap_or(0.0) as f32;
            Move { t: v["t"].as_f64().unwrap_or(0.0), dx: g("dx") * f, dy: g("dy") * f, zoom: g("zoom") * f }
        })
        .collect()
}

/// Each frame's depth, the model loaded once for them all
fn depths(bin: &Path, imgs: &[PathBuf]) -> Result<Vec<Depth>> {
    let outs: Vec<PathBuf> = imgs.iter().map(|p| p.with_extension("f32")).collect();
    let mut cmd = std::process::Command::new(bin);
    cmd.arg("depth-values");
    for (i, o) in imgs.iter().zip(&outs) {
        cmd.arg(i).arg(o);
    }
    let run = cmd.output()?;
    if !run.status.success() {
        return Err(anyhow!("raea-perceive depth-values: {}", String::from_utf8_lossy(&run.stderr).trim()));
    }
    let sizes: Vec<serde_json::Value> = String::from_utf8_lossy(&run.stdout).lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    outs.iter()
        .zip(sizes)
        .map(|(o, s)| {
            let v: Vec<f32> = std::fs::read(o)?.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
            Ok(scaled(s["width"].as_u64().unwrap_or(0) as usize, s["height"].as_u64().unwrap_or(0) as usize, v))
        })
        .collect()
}

/// A render's own depth at each time, from the AOV pass rendered beside it (`raea render EDIT --aov -o NAME.aov`):
/// reverse Z, 1/depth as the model's is, with no model run; None when any frame's is missing
fn rendered(file: &Path, times: &[f64], (w, h): (u32, u32), fps: f64) -> Option<Vec<Depth>> {
    let dir = file.with_extension("aov").join("depth");
    times
        .iter()
        .map(|t| {
            let b = std::fs::read(dir.join(format!("frame-{:05}.f32", (t * fps).round() as i64))).ok()?;
            let v: Vec<f32> = b.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
            // the prepass is the frame's size times RAEA_SS
            let dw = ((v.len() as f64 * w as f64 / h as f64).sqrt().round() as usize).max(1);
            Some(scaled(dw, v.len() / dw, v))
        })
        .collect()
}

/// A render's camera path from its own motion vectors (NAME.aov/motion, `raea render EDIT --aov`), FLOW_FPS times a
/// second over its frames: the median of every pixel's move as the pan, and the median of its move out from the
/// middle over its distance from it as the zoom, both a second. None when the AOV pass has no motion
fn moved(file: &Path, (w, h): (u32, u32), fps: f64, dur: f64) -> Option<Vec<Move>> {
    let dir = file.with_extension("aov").join("motion");
    let mut out = Vec::new();
    for i in 1..((dur * FLOW_FPS).floor() as usize).max(2) {
        let t = i as f64 / FLOW_FPS;
        let Ok(b) = std::fs::read(dir.join(format!("frame-{:05}.f32", (t * fps).round() as i64))) else { break };
        let v: Vec<f32> = b.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        let dw = ((v.len() as f64 / 2.0 * w as f64 / h as f64).sqrt().round() as usize).max(1);
        let (px, py) = (quantile(&v.iter().step_by(2).copied().collect::<Vec<_>>(), 0.5), quantile(&v.iter().skip(1).step_by(2).copied().collect::<Vec<_>>(), 0.5));
        // the zoom from what the pan leaves
        let mut z = Vec::new();
        for (k, m) in v.chunks_exact(2).enumerate() {
            let (rx, ry) = (((k % dw) as f32 + 0.5) / dw as f32 - 0.5, ((k / dw) as f32 + 0.5) / (v.len() / 2 / dw) as f32 - 0.5);
            let r = rx * rx + ry * ry;
            if r > 0.01 {
                z.push(((m[0] - px) * rx + (m[1] - py) * ry) / r);
            }
        }
        let f = fps as f32;
        out.push(Move { t, dx: px * f, dy: py * f, zoom: quantile(&z, 0.5) * f });
    }
    (!out.is_empty()).then_some(out)
}

/// A thing's one vector: its frame's SigLIP 2 patch tokens averaged over the 14 x 14 cells its matte covers half of or
/// more (the cell it covers most, for a thing smaller than a cell), L2-normalised
fn pooled(patches: &[f32], m: &Matte) -> Vec<f32> {
    use raea_sys_columns::SIG;
    let g = 14;
    let mut on = vec![(0usize, 0usize); g * g];
    for y in 0..m.h {
        for x in 0..m.w {
            let c = &mut on[(y * g / m.h) * g + x * g / m.w];
            (c.0, c.1) = (c.0 + usize::from(m.px[y * m.w + x] > 127), c.1 + 1);
        }
    }
    let best = (0..g * g).max_by(|a, b| (on[*a].0 * on[*b].1).cmp(&(on[*b].0 * on[*a].1))).unwrap_or(0);
    let mut v = vec![0f32; SIG];
    for (_, p) in patches.chunks(SIG).enumerate().filter(|(i, _)| on[*i].0 * 2 >= on[*i].1.max(1) || *i == best) {
        v.iter_mut().zip(p).for_each(|(a, b)| *a += b);
    }
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    v.into_iter().map(|x| x / n).collect()
}

/// A thing at `t` from its matte on that frame's depth
fn seen(t: f64, m: &Matte, d: &Depth) -> Option<Seen> {
    let b = bounds(&m.px, m.w, m.h)?;
    let at = |x: f32, y: f32| d.v.get(((y * d.h as f32) as usize).min(d.h.saturating_sub(1)) * d.w + ((x * d.w as f32) as usize).min(d.w.saturating_sub(1))).copied();
    let mut all: Vec<(usize, f32)> = Vec::new();
    for y in 0..m.h {
        for x in 0..m.w {
            if m.px[y * m.w + x] > 127 {
                all.push((y, at((x as f32 + 0.5) / m.w as f32, (y as f32 + 0.5) / m.h as f32)?));
            }
        }
    }
    // its base: its lowest tenth of rows, a row at least
    let (top, low) = (all.iter().map(|x| x.0).min()?, all.iter().map(|x| x.0).max()?);
    let rows = ((low - top + 1) as f32 * 0.1).ceil() as usize;
    let base: Vec<f32> = all.iter().filter(|x| x.0 + rows > low).map(|x| x.1).collect();
    let all: Vec<f32> = all.into_iter().map(|x| x.1).collect();
    let under = (b[3] < FOOT).then(|| {
        let mut v = Vec::new();
        for i in 0..8 {
            for j in 0..8 {
                v.extend(at(b[0] + (b[2] - b[0]) * (i as f32 + 0.5) / 8.0, b[3] + STRIP * (j as f32 + 0.5) / 8.0));
            }
        }
        median(v)
    });
    Some(Seen { t, b, near: median(all)?, base: median(base)?, under: under.flatten() })
}

/// A render's layers as things' mattes at each sample (NAME.layers beside it), keyed by layer
fn layered(file: &Path, times: &[f64]) -> Vec<(String, Vec<(f64, Matte)>)> {
    let Ok(m) = raea_sys_layers::read::read(&file.with_extension("layers")) else { return Vec::new() };
    let fps = raea_sys_columns::media::video(file).map(|d| d.info().fps).unwrap_or(30.0);
    let mut out: Vec<(String, Vec<(f64, Matte)>)> = Vec::new();
    for &t in times {
        // the recorded mask at or before this frame
        let f = (t * fps).round() as i64;
        let Some(mask) = m.frames.keys().filter(|k| **k <= f).max().and_then(|k| m.mask(*k)) else { continue };
        for (j, key) in m.keys.iter().enumerate().filter(|(j, _)| m.kinds[*j] != "text") {
            let px: Vec<u8> = mask.iter().map(|v| if *v as usize == j + 1 { 255 } else { 0 }).collect();
            let share = px.iter().filter(|p| **p > 0).count() as f32 / px.len().max(1) as f32;
            if !(SMALLEST..=LARGEST).contains(&share) {
                continue;
            }
            let matte = Matte { w: m.w, h: m.h, px };
            match out.iter_mut().find(|x| &x.0 == key) {
                Some(x) => x.1.push((t, matte)),
                None => out.push((key.clone(), vec![(t, matte)])),
            }
        }
    }
    out
}

/// The piece's shots, things and camera, and what geometry.rs finds in them
pub fn things(enc: &mut Encoders, file: &Path, marks: &[Mark]) -> Result<Things> {
    let bin = perceive().ok_or_else(|| anyhow!("raea-perceive isn't beside raea or on PATH (RAEA_PERCEIVE): depth, masks and the camera's path come from it"))?;
    let mut v = raea_sys_columns::media::video(file)?;
    let dur = v.info().duration;
    let tmp = std::env::temp_dir().join(format!("raea-fingerprint-things-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    let ran = (|| -> Result<Things> {
        let times: Vec<f64> = (0..((dur * FPS).floor() as usize).max(1)).map(|i| i as f64 / FPS).collect();
        let (mut small, mut imgs, mut frames) = (Vec::new(), Vec::new(), Vec::new());
        for (i, &t) in times.iter().enumerate() {
            let f = v.frame_at(t.min((dur - 1e-3).max(0.0))).map_err(|e| anyhow!("{}: frame at {t:.2} s: {e}", file.display()))?;
            let s = grow::small(f.rgba, f.width, f.height)?;
            let grey: Vec<f32> = s.pixels().map(|p| (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0).collect();
            let mean = grey.iter().sum::<f32>() / grey.len().max(1) as f32;
            let detail = (grey.iter().map(|g| (g - mean).powi(2)).sum::<f32>() / grey.len().max(1) as f32).sqrt();
            let p = tmp.join(format!("f{i:04}.png"));
            s.save(&p)?;
            imgs.push(p);
            small.push(s);
            frames.push(Frame { t, detail, depth: None });
        }
        let info = v.info();
        let depth = match rendered(file, &times, (info.width, info.height), info.fps) {
            Some(d) => d,
            None => depths(&bin, &imgs)?,
        };
        for (f, d) in frames.iter_mut().zip(&depth) {
            f.depth = Some(quantile(&d.v, 0.95) - quantile(&d.v, 0.05));
        }
        let near = |t: f64| ((t * FPS).round() as usize).min(times.len() - 1);
        // the things: a render's layers, then each mark followed
        let mut found: Vec<(String, String, Vec<(f64, Matte)>)> = layered(file, &times).into_iter().map(|(k, m)| (k, "layer".to_string(), m)).collect();
        for m in marks {
            let t = near(m.t) as f64 / FPS;
            if let Some(f) = grow::follow(file, t, m.a, m.b, m.region, FPS)? {
                found.push((m.name.clone(), "mark".into(), f.into_iter().filter_map(|(t, x)| Some((t, x?))).collect()));
            }
        }
        let mut all = Vec::new();
        for (name, from, mattes) in found {
            let seen: Vec<Seen> = mattes.iter().filter_map(|(t, m)| seen(*t, m, &depth[near(*t)])).collect();
            let Some(big) = seen.iter().max_by(|a, b| ((a.b[2] - a.b[0]) * (a.b[3] - a.b[1])).total_cmp(&((b.b[2] - b.b[0]) * (b.b[3] - b.b[1])))) else { continue };
            let s = &small[near(big.t)];
            let Some((_, m)) = mattes.iter().find(|x| (x.0 - big.t).abs() < 1e-6) else { continue };
            let embed = pooled(&enc.patches(&[raea_sys_columns::media::squash(s.as_raw(), s.width() as usize, s.height() as usize)])?, m);
            all.push(Thing { name, from, seen, embed });
        }
        let cam = moved(file, (info.width, info.height), info.fps, dur).unwrap_or_else(|| camera(&bin, file));
        let mut edges = vec![0.0];
        for c in grow::cuts(&bin, file) {
            if c > 0.0 && c < dur {
                edges.push(c);
            }
        }
        edges.push(dur);
        let mut shots = Vec::new();
        let mut issues = geometry::empty(&frames, dur);
        for w in edges.windows(2) {
            let (a, b) = (w[0], w[1]);
            let inside = |t: f64| t >= a && t < b;
            let camera: Vec<Move> = cam.iter().filter(|m| inside(m.t)).cloned().collect();
            let things: Vec<Thing> = all.iter().map(|x| Thing { seen: x.seen.iter().filter(|s| inside(s.t)).cloned().collect(), ..x.clone() }).filter(|x| !x.seen.is_empty()).collect();
            issues.extend(geometry::speed_change(&camera));
            for (i, x) in things.iter().enumerate() {
                issues.extend(geometry::floating(&x.name, &x.seen));
                for y in &things[i + 1..] {
                    issues.extend(geometry::passing((&x.name, &x.seen), (&y.name, &y.seen)));
                }
            }
            shots.push(Shot { a, b, camera, things });
        }
        issues.sort_by(|x, y| x.at.total_cmp(&y.at));
        Ok(Things { file: file.display().to_string(), dur, shots, frames, issues })
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ran
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_things_base_and_the_ground_under_it() {
        // a 10 x 10 depth: the floor nearer toward the foot, a box standing on it, and one hanging over far wall
        let mut v = vec![0.0f32; 100];
        for y in 0..10 {
            for x in 0..10 {
                v[y * 10 + x] = 0.1 * y as f32;
            }
        }
        let d = Depth { w: 10, h: 10, v };
        let mut px = vec![0u8; 100];
        for y in 4..7 {
            for x in 3..6 {
                px[y * 10 + x] = 255;
            }
        }
        let m = Matte { w: 10, h: 10, px };
        let s = seen(1.0, &m, &d).unwrap();
        assert_eq!(s.b, [0.3, 0.4, 0.6, 0.7]);
        assert!((s.base - 0.6).abs() < 1e-5 && s.under.is_some_and(|u| (u - 0.7).abs() < 1e-5), "{s:?}");
        let mut foot = vec![0u8; 100];
        foot[95] = 255;
        foot[85] = 255;
        assert_eq!(seen(1.0, &Matte { w: 10, h: 10, px: foot }, &d).unwrap().under, None, "its base is at the frame's foot");
    }

    #[test]
    fn a_render_s_camera_is_its_own_motion() {
        // a 40 x 20 pass at 8 fps: frames 1-8 pan right a hundredth of the frame a frame, 9-16 zoom in 2% a frame
        let dir = std::env::temp_dir().join(format!("raea-things-moved-{}", std::process::id()));
        let m = dir.join("cut.aov/motion");
        std::fs::create_dir_all(&m).unwrap();
        for f in 1..=16 {
            let v: Vec<f32> = (0..800).flat_map(|k| {
                let (rx, ry) = (((k % 40) as f32 + 0.5) / 40.0 - 0.5, ((k / 40) as f32 + 0.5) / 20.0 - 0.5);
                if f <= 8 { [0.01, 0.0] } else { [0.02 * rx, 0.02 * ry] }
            }).collect();
            std::fs::write(m.join(format!("frame-{f:05}.f32")), v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()).unwrap();
        }
        let c = moved(&dir.join("cut.mp4"), (40, 20), 8.0, 2.0).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(c.len(), 15);
        assert!((c[0].dx - 0.08).abs() < 1e-5 && c[0].zoom.abs() < 1e-5, "{:?}", c[0]);
        assert!((c[10].zoom - 0.16).abs() < 1e-3 && c[10].dx.abs() < 0.005 && c[10].dy.abs() < 0.005, "{:?}", c[10]);
        assert_eq!(moved(&dir.join("none.mp4"), (40, 20), 8.0, 2.0).map(|c| c.len()), None, "no pass, no path");
    }

    /// `RAEA_CAMERA_MEDIA=FILE cargo test -p raea-sys-fingerprint-things camera_two_ways -- --ignored --nocapture`: a
    /// render's camera from its motion vectors beside the flow's, FILE.aov beside it
    #[test]
    #[ignore]
    fn camera_two_ways() {
        let f = PathBuf::from(std::env::var("RAEA_CAMERA_MEDIA").expect("RAEA_CAMERA_MEDIA"));
        let d = raea_sys_columns::media::video(&f).unwrap();
        let i = d.info();
        let (own, flow) = (moved(&f, (i.width, i.height), i.fps, i.duration).expect("FILE.aov/motion"), camera(&perceive().expect("raea-perceive"), &f));
        for (a, b) in own.iter().zip(&flow) {
            eprintln!("{:.3} own {:+.3} {:+.3} {:+.3}  flow {:.3} {:+.3} {:+.3} {:+.3}", a.t, a.dx, a.dy, a.zoom, b.t, b.dx, b.dy, b.zoom);
        }
    }

    #[test]
    fn a_thing_s_vector_is_its_cells_patches() {
        use raea_sys_columns::SIG;
        // patch i carries i in its first value; a 28 x 28 matte on over cells (2, 3) and (3, 3), and a quarter of (5, 5)
        let patches: Vec<f32> = (0..196 * SIG).map(|j| if j % SIG == 0 { (j / SIG) as f32 } else { 0.0 }).collect();
        let mut px = vec![0u8; 28 * 28];
        for (x, y) in [(4, 6), (5, 6), (4, 7), (5, 7), (6, 6), (7, 6), (6, 7), (7, 7), (10, 10)] {
            px[y * 28 + x] = 255;
        }
        let v = pooled(&patches, &Matte { w: 28, h: 28, px });
        // cells 3 * 14 + 2 = 44 and 45 in, 75 (a quarter) out: 89 over its length
        assert!((v[0] - 1.0).abs() < 1e-6, "{}", v[0]);
        let one = pooled(&patches, &Matte { w: 28, h: 28, px: { let mut p = vec![0u8; 784]; p[10 * 28 + 10] = 255; p } });
        assert!((one[0] - 1.0).abs() < 1e-6, "a thing under a cell is its cell's patch");
    }

    /// `RAEA_THINGS_MEDIA=FILE cargo test -p raea-sys-fingerprint-things things_on -- --ignored --nocapture`: a render's
    /// things, each vector's length and how alike each two are
    #[test]
    #[ignore]
    fn things_on_a_real_render() {
        let f = PathBuf::from(std::env::var("RAEA_THINGS_MEDIA").expect("RAEA_THINGS_MEDIA"));
        let t = things(&mut Encoders::load().unwrap(), &f, &[]).unwrap();
        let all: Vec<&Thing> = t.shots.iter().flat_map(|s| &s.things).collect();
        for (i, a) in all.iter().enumerate() {
            let near: Vec<String> = all.iter().map(|b| format!("{:.2}", a.embed.iter().zip(&b.embed).map(|(x, y)| x * y).sum::<f32>())).collect();
            eprintln!("{i} {} {} [{}]", a.name, a.embed.len(), near.join(" "));
        }
        assert!(all.iter().all(|a| a.embed.len() == raea_sys_columns::SIG));
    }
}
