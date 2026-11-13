//! A mark grown into its thing (docs/fingerprint.feature FP11b): the region a note's click, box or lasso names is
//! tracked by EdgeTAM (raea-perceive track) forward from the note's frame to the end of its label and back to its
//! start, so one mark labels every frame of it. A frame where EdgeTAM loses the thing is dropped; the row's region is
//! the box around the frames it kept. Without raea-perceive the mark's own region stands.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

/// Frames a second the thing is tracked at
const FPS: f64 = 4.0;
/// A tracked frame's longest side, pixels
const SIDE: u32 = 512;
/// Less of the frame than this, and EdgeTAM has lost the thing
const LOST: f64 = 0.0005;
/// A click marks this far around it, of the frame
const CLICK: f32 = 0.05;

#[derive(Clone, Debug, PartialEq)]
pub struct Grown {
    pub region: [f32; 4],
    pub kept: usize,
    pub lost: usize,
}

/// raea-perceive: $RAEA_PERCEIVE, else beside this raea, else on PATH
pub fn perceive() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RAEA_PERCEIVE").filter(|v| !v.is_empty()) {
        return Some(p.into());
    }
    let name = if cfg!(windows) { "raea-perceive.exe" } else { "raea-perceive" };
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe().and_then(|e| e.canonicalize()) {
        dirs.extend(exe.parent().map(Path::to_path_buf));
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

/// The box around a matte's pixels at 128 or more, x0,y0,x1,y1 in 0-1; None when too little of it is on
pub fn bounds(matte: &[u8], w: usize, h: usize) -> Option<[f32; 4]> {
    let (mut x0, mut y0, mut x1, mut y1, mut n) = (w, h, 0, 0, 0usize);
    for y in 0..h {
        for x in 0..w {
            if matte[y * w + x] > 127 {
                (x0, y0, x1, y1, n) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1), n + 1);
            }
        }
    }
    (n as f64 / (w * h).max(1) as f64 >= LOST).then(|| [x0 as f32 / w as f32, y0 as f32 / h as f32, x1 as f32 / w as f32, y1 as f32 / h as f32])
}

/// The box around boxes
pub fn union(a: Option<[f32; 4]>, b: [f32; 4]) -> [f32; 4] {
    let Some(mut u) = a else { return b };
    for i in 0..2 {
        u[i] = u[i].min(b[i]);
        u[i + 2] = u[i + 2].max(b[i + 2]);
    }
    u
}

/// A matte EdgeTAM drew: w x h, a pixel on at 128 or more
#[derive(Clone, Debug, PartialEq)]
pub struct Matte {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

/// The note's frames from `from` toward `to` (either way) at `fps`, the first at `from`, as PNGs in `dir`, with their times
fn frames(media: &Path, from: f64, to: f64, fps: f64, dir: &Path) -> Result<Vec<(f64, PathBuf)>> {
    let mut d = raea_sys_columns::media::video(media)?;
    let end = (d.info().duration - 1e-3).max(0.0);
    let n = ((to - from).abs() * fps).floor() as usize + 1;
    let step = if to >= from { 1.0 / fps } else { -1.0 / fps };
    std::fs::create_dir_all(dir)?;
    let mut out = Vec::new();
    for i in 0..n {
        let t = (from + i as f64 * step).clamp(0.0, end);
        let f = d.frame_at(t).map_err(|e| anyhow!("{}: frame at {t:.2} s: {e}", media.display()))?;
        let p = dir.join(format!("f{i:03}.png"));
        small(f.rgba, f.width, f.height)?.save(&p)?;
        out.push((t, p));
    }
    Ok(out)
}

/// A frame at SIDE on its longest side
pub fn small(rgba: Vec<u8>, w: u32, h: u32) -> Result<image::RgbaImage> {
    let img = image::RgbaImage::from_raw(w, h, rgba).ok_or_else(|| anyhow!("frame size"))?;
    let s = SIDE as f32 / w.max(h) as f32;
    Ok(image::imageops::resize(&img, ((w as f32 * s) as u32).max(1), ((h as f32 * s) as u32).max(1), image::imageops::FilterType::Triangle))
}

/// One run of EdgeTAM from `seed` on the first of `imgs`: each frame's matte, None where it lost the thing
fn track(bin: &Path, seed: [f32; 4], imgs: &[PathBuf], out: &Path) -> Result<Vec<Option<Matte>>> {
    let b = format!("{},{},{},{}", seed[0], seed[1], (seed[2] - seed[0]).max(0.001), (seed[3] - seed[1]).max(0.001));
    let run = std::process::Command::new(bin).arg("track").arg(out).arg(&b).args(imgs).output()?;
    if !run.status.success() {
        return Err(anyhow!("raea-perceive track: {}", String::from_utf8_lossy(&run.stderr).trim()));
    }
    let mut mattes = Vec::new();
    for p in imgs {
        let m = image::open(out.join(p.file_name().unwrap_or_default()))?.to_luma8();
        let m = Matte { w: m.width() as usize, h: m.height() as usize, px: m.into_raw() };
        mattes.push(bounds(&m.px, m.w, m.h).is_some().then_some(m));
    }
    Ok(mattes)
}

/// The mark's thing followed over a..b at `fps` from the note's moment `t`, forward then back: each frame's time and
/// matte (None where EdgeTAM lost it); Ok(None) without raea-perceive
pub fn follow(media: &Path, t: f64, a: f64, b: f64, seed: [f32; 4], fps: f64) -> Result<Option<Vec<(f64, Option<Matte>)>>> {
    let Some(bin) = perceive() else { return Ok(None) };
    let tmp = std::env::temp_dir().join(format!("raea-fingerprint-follow-{}-{}", std::process::id(), (t * 1000.0) as i64));
    let ran = (|| -> Result<Vec<(f64, Option<Matte>)>> {
        let mut out = Vec::new();
        for (i, end) in [b, a].into_iter().enumerate() {
            if (end - t).abs() * fps < 1.0 && i == 1 {
                continue;
            }
            let dir = tmp.join(format!("{i}"));
            let imgs = frames(media, t, end, fps, &dir.join("in"))?;
            let paths: Vec<PathBuf> = imgs.iter().map(|x| x.1.clone()).collect();
            // the note's own frame is the first of both runs: kept once
            out.extend(imgs.iter().map(|x| x.0).zip(track(&bin, seed, &paths, &dir.join("out"))?).skip(i));
        }
        out.sort_by(|x, y| x.0.total_cmp(&y.0));
        Ok(out)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ran.map(Some)
}

/// The mark's region tracked over a..b from the note's moment `t`; Ok(None) without raea-perceive
pub fn grow(media: &Path, t: f64, a: f64, b: f64, seed: [f32; 4]) -> Result<Option<Grown>> {
    let Some(seen) = follow(media, t, a, b, seed, FPS)? else { return Ok(None) };
    let mut g = Grown { region: seed, kept: 0, lost: 0 };
    let mut region = None;
    for (_, m) in &seen {
        match m.as_ref().and_then(|m| bounds(&m.px, m.w, m.h)) {
            Some(r) => (region, g.kept) = (Some(union(region, r)), g.kept + 1),
            None => g.lost += 1,
        }
    }
    g.region = region.unwrap_or(seed);
    Ok(Some(g))
}

/// The region a note's marks name: the box around its marks, boxes and lassos (a click as a small square around it).
/// Arrows and strikes are instructions for the agent, never a region
pub fn marked(pin: &serde_json::Value) -> Option<[f32; 4]> {
    let mut r: Option<[f32; 4]> = None;
    for m in pin["marks"].as_array().into_iter().flatten() {
        let reach = match m["kind"].as_str() {
            Some("mark") => CLICK,
            Some("box" | "lasso") => 0.0,
            _ => continue,
        };
        for p in m["points"].as_array().into_iter().flatten() {
            let (Some(x), Some(y)) = (p[0].as_f64(), p[1].as_f64()) else { continue };
            let (x, y) = (x as f32, y as f32);
            r = Some(union(r, [(x - reach).max(0.0), (y - reach).max(0.0), (x + reach).min(1.0), (y + reach).min(1.0)]));
        }
    }
    r.filter(|a| a[2] > a[0] && a[3] > a[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matte_s_box_and_a_lost_one() {
        let (w, h) = (10, 10);
        let mut m = vec![0u8; w * h];
        for y in 2..5 {
            for x in 6..9 {
                m[y * w + x] = 255;
            }
        }
        assert_eq!(bounds(&m, w, h), Some([0.6, 0.2, 0.9, 0.5]));
        assert_eq!(bounds(&vec![0u8; w * h], w, h), None, "nothing on is a lost frame");
        assert_eq!(union(Some([0.1, 0.5, 0.2, 0.6]), [0.6, 0.2, 0.9, 0.5]), [0.1, 0.2, 0.9, 0.6]);
    }

    #[test]
    fn marks_name_a_region_and_arrows_dont() {
        use serde_json::json;
        let pin = json!({"kind": "place", "x": 0.5, "y": 0.5, "marks": [
            {"kind": "box", "points": [[0.6, 0.7], [0.2, 0.3]]},
            {"kind": "mark", "points": [[0.9, 0.1]]},
            {"kind": "arrow", "points": [[0.0, 0.0], [1.0, 1.0]]}
        ]});
        let r = marked(&pin).unwrap();
        let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(near(r[0], 0.2) && near(r[1], 0.05) && near(r[2], 0.95) && near(r[3], 0.7), "{r:?}");
        assert_eq!(marked(&json!({"marks": [{"kind": "strike", "points": [[0.1, 0.1], [0.9, 0.9]]}]})), None, "a strike is for the agent");
        assert_eq!(marked(&json!({"kind": "place", "x": 0.5, "y": 0.5})), None, "a pin is never a mark");
    }
}

#[cfg(test)]
mod real {
    /// `RAEA_GROW_MEDIA=FILE cargo test -p raea-sys-fingerprint-use grows -- --ignored`: a box in the middle of FILE at 1 s,
    /// grown over 0..2 s by raea-perceive
    #[test]
    #[ignore]
    fn grows_on_a_real_render() {
        let f = std::path::PathBuf::from(std::env::var("RAEA_GROW_MEDIA").expect("RAEA_GROW_MEDIA"));
        let g = super::grow(&f, 1.0, 0.0, 2.0, [0.4, 0.4, 0.6, 0.6]).expect("grown").expect("raea-perceive");
        eprintln!("{g:?}");
        assert!(g.kept > 0, "EdgeTAM kept no frame");
        assert!(g.region[2] > g.region[0] && g.region[3] > g.region[1]);
    }
}
