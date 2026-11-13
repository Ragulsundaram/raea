//! Where a picture flag is (docs/fingerprint.feature FP12a): patch heads bootstrapped from FP0f's local defects put
//! into frames of real work, and a scored piece's moments placed in the frame and read through its layer pass into
//! the layers they cover. Only a head that points held out (patch.rs POINTS) says where.

use anyhow::Result;
use raea_sys_columns::damage::{LOCAL, Noise, Square, local};
use raea_sys_columns::media::{self, SIDE};
use raea_sys_columns::{Encoders, FPS};
use raea_sys_fingerprint_use::patch::{self, CELLS, Frame, GRID, PatchHead};
use raea_sys_fingerprint_use::score::{Scored, Where};
use std::path::{Path, PathBuf};

/// The cells of the 14 x 14 grid a box in the 224 x 224 frame covers by half or more
fn cells(b: Square) -> Vec<bool> {
    let cell = SIDE / GRID;
    (0..CELLS)
        .map(|i| {
            let (x, y) = ((i % GRID) * cell, (i / GRID) * cell);
            let over = |a: usize, s: usize| (a + cell).min(s + b.side).saturating_sub(a.max(s));
            over(x, b.x) * over(y, b.y) * 2 >= cell * cell
        })
        .collect()
}

/// The mean change a defect has to make in its box, in levels of 255, to count as one
pub const VISIBLE: f64 = 6.0;

/// The mean change between two frames inside a box, in levels of 255
fn changed(a: &media::Rgb, b: &media::Rgb, s: Square) -> f64 {
    let px = s.pixels();
    px.iter().map(|&i| (0..3).map(|c| (a[i * 3 + c] as f64 - b[i * 3 + c] as f64).abs()).sum::<f64>() / 3.0).sum::<f64>() / px.len().max(1) as f64
}

/// `per` frames from each file, evenly through it: each clean, then with each of LOCAL's defects in a random box; a
/// defect that left its box looking the same (under VISIBLE, as blur on a clear sky) is no defect, and is left out
pub fn frames(enc: &mut Encoders, files: &[PathBuf], per: usize, seed: u64, say: &mut dyn FnMut(String)) -> Result<Vec<Frame>> {
    let (mut rng, mut noise, mut out) = (fastrand::Rng::with_seed(seed), Noise::new(seed), Vec::new());
    for (n, file) in files.iter().enumerate() {
        let dur = media::duration(file).unwrap_or(0.0);
        let source = file.display().to_string();
        for k in 0..per {
            let t = dur * (k as f64 + 0.5) / per as f64;
            let Some(f) = media::frames(file, FPS, t, 1.0 / FPS)?.into_iter().next() else { continue };
            let boxes: Vec<Square> = LOCAL.iter().map(|_| Square::random(&mut rng)).collect();
            let mut batch = vec![f.clone()];
            batch.extend(LOCAL.iter().zip(&boxes).map(|((kind, _), b)| local(&f, kind, *b, &mut noise)));
            let p = enc.patches(&batch)?;
            let one = CELLS * raea_sys_columns::SIG;
            out.push(Frame { source: source.clone(), axis: "*".into(), kind: "clean".into(), patches: p[..one].to_vec(), inside: vec![false; CELLS] });
            for (j, ((kind, axis), b)) in LOCAL.iter().zip(&boxes).enumerate().filter(|(j, (_, b))| changed(&f, &batch[j + 1], **b) >= VISIBLE) {
                out.push(Frame { source: source.clone(), axis: (*axis).into(), kind: (*kind).into(), patches: p[(j + 1) * one..(j + 2) * one].to_vec(), inside: cells(*b) });
            }
        }
        say(format!("patch heads: source {}/{} {}", n + 1, files.len(), file.display()));
    }
    Ok(out)
}

/// The frames a source gives the patch heads at most: each is six frames' patch tokens (2.4 MB) held until training
pub const PER: usize = 6;

/// `raea fingerprint bootstrap`'s patch heads: frames of each file with LOCAL's defects, a head for each axis held out
/// by source and then trained on all, kept in the fingerprint's folder (patch.rs FILE); what report.json says of them
pub fn bootstrap(enc: &mut Encoders, dir: &Path, files: &[PathBuf], per: usize, say: &mut dyn FnMut(String)) -> Result<serde_json::Value> {
    let heads = patch::train(&frames(enc, files, per.min(PER), 0, say)?, 0);
    patch::save(dir, &heads)?;
    Ok(heads.iter().map(|h| serde_json::json!({"axis": h.axis, "pointing": h.pointing, "by_kind": h.by_kind, "points": h.points(), "frames": h.frames})).collect())
}

/// The layers a box covers in a render's layer pass (NAME.layers beside it) at time t, by name, most of the box first;
/// a layer under a tenth of the box is left out
pub fn layers(file: &Path, t: f64, b: [f64; 4]) -> Vec<String> {
    let Ok(m) = raea_sys_layers::read::read(&file.with_extension("layers")) else { return Vec::new() };
    let fps = media::video(file).map(|d| d.info().fps).unwrap_or(30.0);
    let f = (t * fps).round() as i64;
    let Some(mask) = m.frames.keys().filter(|k| **k <= f).max().and_then(|k| m.mask(*k)) else { return Vec::new() };
    let (x0, y0, x1, y1) = ((b[0] * m.w as f64) as usize, (b[1] * m.h as f64) as usize, (b[2] * m.w as f64) as usize, (b[3] * m.h as f64) as usize);
    let mut count = vec![0usize; m.keys.len() + 1];
    for y in y0..y1.min(m.h) {
        for x in x0..x1.min(m.w) {
            count[mask[y * m.w + x] as usize] += 1;
        }
    }
    let all = ((x1 - x0) * (y1 - y0)).max(1);
    let mut by: Vec<(usize, usize)> = count.iter().enumerate().skip(1).filter(|(_, n)| **n * 10 >= all).map(|(v, n)| (v, *n)).collect();
    by.sort_by(|a, b| b.1.cmp(&a.1));
    by.into_iter().map(|(v, _)| Some(&m.names[v - 1]).filter(|n| !n.is_empty()).unwrap_or(&m.keys[v - 1]).clone()).collect()
}

/// Each moment whose axis has a patch head that points gets where it is, from its middle frame
pub fn locate(enc: &mut Encoders, file: &Path, heads: &[PatchHead], s: &mut Scored) -> Result<()> {
    for m in s.moments.iter_mut() {
        let Some(h) = heads.iter().find(|h| h.axis == m.axis && h.points()) else { continue };
        let t = (m.a + m.b) / 2.0;
        let Some(f) = media::frames(file, FPS, t, 1.0 / FPS)?.into_iter().next() else { continue };
        let region = patch::region(&patch::heat(&h.head, &enc.patches(&[f])?));
        m.at = Some(Where { t, region, layers: layers(file, t, region) });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `RAEA_PATCHES=LIST OUT=DIR cargo test --release -p raea-sys-fingerprint-things patch_heads_on -- --ignored
    /// --nocapture`: the bootstrap's patch heads on the renders LIST names (one path a line), held out by source
    #[test]
    #[ignore]
    fn patch_heads_on_this_machine() {
        let list = std::env::var("RAEA_PATCHES").expect("RAEA_PATCHES");
        let files: Vec<PathBuf> = std::fs::read_to_string(list).unwrap().lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect();
        let out = PathBuf::from(std::env::var("OUT").expect("OUT"));
        std::fs::create_dir_all(&out).unwrap();
        let mut enc = Encoders::load().unwrap();
        let said = bootstrap(&mut enc, &out, &files, PER, &mut |s| eprintln!("{s}")).unwrap();
        eprintln!("{}", serde_json::to_string_pretty(&said).unwrap());
    }

    /// `RAEA_LAYERS_MEDIA=FILE RAEA_LAYERS_AT=t,x,y cargo test -p raea-sys-fingerprint-things layers_under -- --ignored
    /// --nocapture`: the layers under a pin at x,y (0-1) at t s, as FP5a's reading keeps them
    #[test]
    #[ignore]
    fn layers_under_a_pin() {
        let f = PathBuf::from(std::env::var("RAEA_LAYERS_MEDIA").expect("RAEA_LAYERS_MEDIA"));
        let a: Vec<f64> = std::env::var("RAEA_LAYERS_AT").expect("RAEA_LAYERS_AT").split(',').filter_map(|v| v.parse().ok()).collect();
        let l = layers(&f, a[0], [a[1] - 0.01, a[2] - 0.01, a[1] + 0.01, a[2] + 0.01]);
        eprintln!("{l:?}");
        assert!(!l.is_empty(), "no layer under the pin");
    }

    #[test]
    fn a_box_covers_the_cells_it_is_mostly_over() {
        // 64 px from (32, 48): cells 2..6 across (32 px is cell 2's left edge) and rows 3..7 (48 to 112)
        let c = cells(Square { x: 32, y: 48, side: 64 });
        let on: Vec<(usize, usize)> = (0..CELLS).filter(|i| c[*i]).map(|i| (i % GRID, i / GRID)).collect();
        assert_eq!(on.len(), 16);
        assert_eq!((on[0], on[15]), ((2, 3), (5, 6)));
    }
}
