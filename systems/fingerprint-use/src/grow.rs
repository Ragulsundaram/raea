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

#[derive(Clone, Debug, PartialEq)]
pub struct Grown {
    pub region: [f32; 4],
    pub kept: usize,
    pub lost: usize,
}

/// raea-perceive: $RAEA_PERCEIVE, else beside this raea, else on PATH
pub fn perceive() -> Option<PathBuf> {
    let name = if cfg!(windows) { "raea-perceive.exe" } else { "raea-perceive" };
    let beside = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()).and_then(|e| Some(e.parent()?.join(name)));
    let on_path = std::env::var_os("PATH").into_iter().flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).map(|d| d.join(name));
    std::env::var_os("RAEA_PERCEIVE").filter(|v| !v.is_empty()).map(PathBuf::from).into_iter().chain(beside).chain(on_path).find(|p| p.is_file())
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
    a.map_or(b, |a| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
}

/// The note's frames from `from` toward `to` (either way), the first at `from`, as PNGs in `dir`
fn frames(media: &Path, from: f64, to: f64, dir: &Path) -> Result<Vec<PathBuf>> {
    let mut d = raea_sys_columns::media::video(media)?;
    let end = (d.info().duration - 1e-3).max(0.0);
    let n = ((to - from).abs() * FPS).floor() as usize + 1;
    let step = if to >= from { 1.0 / FPS } else { -1.0 / FPS };
    std::fs::create_dir_all(dir)?;
    let mut out = Vec::new();
    for i in 0..n {
        let t = (from + i as f64 * step).clamp(0.0, end);
        let f = d.frame_at(t).map_err(|e| anyhow!("{}: frame at {t:.2} s: {e}", media.display()))?;
        let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba).ok_or_else(|| anyhow!("frame size"))?;
        let s = SIDE as f32 / f.width.max(f.height) as f32;
        let small = image::imageops::resize(&img, ((f.width as f32 * s) as u32).max(1), ((f.height as f32 * s) as u32).max(1), image::imageops::FilterType::Triangle);
        let p = dir.join(format!("f{i:03}.png"));
        small.save(&p)?;
        out.push(p);
    }
    Ok(out)
}

/// One run of EdgeTAM from `seed` on the first of `imgs`: each frame's box, None where it lost the thing
fn track(bin: &Path, seed: [f32; 4], imgs: &[PathBuf], out: &Path) -> Result<Vec<Option<[f32; 4]>>> {
    let b = format!("{},{},{},{}", seed[0], seed[1], (seed[2] - seed[0]).max(0.001), (seed[3] - seed[1]).max(0.001));
    let run = std::process::Command::new(bin).arg("track").arg(out).arg(&b).args(imgs).output()?;
    if !run.status.success() {
        return Err(anyhow!("raea-perceive track: {}", String::from_utf8_lossy(&run.stderr).trim()));
    }
    let mut boxes = Vec::new();
    for p in imgs {
        let m = image::open(out.join(p.file_name().unwrap_or_default()))?.to_luma8();
        boxes.push(bounds(m.as_raw(), m.width() as usize, m.height() as usize));
    }
    Ok(boxes)
}

/// The mark's region tracked over a..b from the note's moment `t`; Ok(None) without raea-perceive
pub fn grow(media: &Path, t: f64, a: f64, b: f64, seed: [f32; 4]) -> Result<Option<Grown>> {
    let Some(bin) = perceive() else { return Ok(None) };
    let tmp = std::env::temp_dir().join(format!("raea-fingerprint-grow-{}", std::process::id()));
    let ran = (|| -> Result<Grown> {
        let mut g = Grown { region: seed, kept: 0, lost: 0 };
        let mut region = None;
        for (i, end) in [b, a].into_iter().enumerate() {
            if (end - t).abs() * FPS < 1.0 && i == 1 {
                continue;
            }
            let dir = tmp.join(format!("{i}"));
            let imgs = frames(media, t, end, &dir.join("in"))?;
            for x in track(&bin, seed, &imgs, &dir.join("out"))? {
                match x {
                    Some(r) => (region, g.kept) = (Some(union(region, r)), g.kept + 1),
                    None => g.lost += 1,
                }
            }
        }
        g.region = region.unwrap_or(seed);
        Ok(g)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ran.map(Some)
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
