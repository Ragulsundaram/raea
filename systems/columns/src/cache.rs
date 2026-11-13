//! Columns are a cache (docs/platform.feature: derived files are a cache): one file per media sha256 and STEP under
//! $RAEA_COLUMNS, else ~/.cache/raea-fingerprint/columns. The file: a JSON header line (media, dur, step, each
//! column's length), then the columns in order as little-endian fp16.

use crate::{Columns, STEP};
use anyhow::Result;
use half::f16;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The file's first line
#[derive(Serialize, Deserialize)]
struct Head {
    media: String,
    dur: f64,
    step: u32,
    /// each column's length, in the order saved
    lens: Vec<usize>,
}

pub fn root() -> PathBuf {
    crate::under("RAEA_COLUMNS", ".cache/raea-fingerprint")
}

fn path(media: &str) -> PathBuf {
    root().join("columns").join(format!("{media}-{STEP}.cols"))
}

pub fn save(c: &Columns) -> Result<()> {
    let cols = [&c.sig, &c.look, &c.clap, &c.yam, &c.level];
    let head = Head { media: c.media.clone(), dur: c.dur, step: STEP, lens: cols.iter().map(|v| v.len()).collect() };
    let mut b = serde_json::to_vec(&head)?;
    b.push(b'\n');
    for v in cols {
        for x in v.iter() {
            b.extend(f16::from_f32(*x).to_le_bytes());
        }
    }
    let p = path(&c.media);
    std::fs::create_dir_all(p.parent().unwrap())?;
    let tmp = p.with_extension("part");
    std::fs::write(&tmp, b)?;
    std::fs::rename(tmp, p)?;
    Ok(())
}

pub fn load(media: &str) -> Option<Columns> {
    let b = std::fs::read(path(media)).ok()?;
    let nl = b.iter().position(|&x| x == b'\n')?;
    let head: Head = serde_json::from_slice(&b[..nl]).ok()?;
    if head.lens.len() != 5 || head.step != STEP {
        return None;
    }
    let mut at = nl + 1;
    let mut cols: Vec<Vec<f32>> = Vec::new();
    for &n in &head.lens {
        let end = at + 2 * n;
        let raw = b.get(at..end)?;
        cols.push(raw.as_chunks::<2>().0.iter().map(|p| f16::from_le_bytes(*p).to_f32()).collect());
        at = end;
    }
    let mut it = cols.into_iter();
    Some(Columns {
        media: media.to_string(),
        dur: head.dur,
        sig: it.next()?,
        look: it.next()?,
        clap: it.next()?,
        yam: it.next()?,
        level: it.next()?,
    })
}
