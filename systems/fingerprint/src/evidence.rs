//! The evidence ledger (FP2c): one row per label, in evidence.jsonl, with the window's features for every modality in
//! evidence.f16 at the row's offset (about 4.7 KB a row). A row is a label on one axis, or with axis "*" a label on
//! every axis (an approval, or a clean window beside its damaged copies). Rows are the only thing a team shares
//! (FP8); heads are always retrained from them.

use anyhow::Result;
use half::f16;
use raea_sys_columns::window::Modality;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Every modality's features, in Modality::ALL's order
pub fn width() -> usize {
    Modality::ALL.iter().map(|m| m.dim()).sum()
}

/// Where a modality's features sit in a row's vector
pub fn slice(m: Modality) -> std::ops::Range<usize> {
    let mut at = 0;
    for x in Modality::ALL {
        if x == m {
            return at..at + x.dim();
        }
        at += x.dim();
    }
    0..0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {
    /// the media's sha256 and the span, seconds
    pub media: String,
    pub a: f64,
    pub b: f64,
    /// the axis, or "*" for every axis
    pub axis: String,
    /// true: the axis's defect is there; false: it isn't
    pub bad: bool,
    #[serde(default = "one")]
    pub weight: f32,
    pub who: String,
    /// damage, cheat, teach, note, approval, pair, postmortem, eye:<name>
    pub source: String,
    /// what held-out scores hold out by (a project, or a source file in a bootstrap)
    pub project: String,
    /// the damage or cheat kind, for a bootstrap's rows
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// a mark's box in 0-1 frame coordinates (FP11)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<[f32; 4]>,
    /// the person's own words, when the label came with some
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// the row's index in evidence.f16
    pub at: u64,
    /// the encoders its features came from, when they aren't the fingerprint's own any more (FP8c): such a row waits
    /// to be re-embedded or re-labelled, and no head trains on it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
}

fn one() -> f32 {
    1.0
}

#[derive(Default)]
pub struct Ledger {
    pub rows: Vec<Row>,
    pub feats: Vec<f32>,
}

impl Ledger {
    pub fn features(&self, i: usize) -> &[f32] {
        let w = width();
        let at = self.rows[i].at as usize * w;
        &self.feats[at..at + w]
    }

    pub fn modality(&self, i: usize, m: Modality) -> &[f32] {
        &self.features(i)[slice(m)]
    }

    /// A new row with its features
    pub fn push(&mut self, mut r: Row, feats: &[f32]) {
        assert_eq!(feats.len(), width(), "a row carries every modality");
        r.at = (self.feats.len() / width()) as u64;
        self.feats.extend_from_slice(feats);
        self.rows.push(r);
    }

    pub fn read(dir: &Path) -> Result<Ledger> {
        let (j, f) = (dir.join("evidence.jsonl"), dir.join("evidence.f16"));
        if !j.is_file() {
            return Ok(Ledger::default());
        }
        let rows = std::fs::read_to_string(j)?.lines().filter(|l| !l.trim().is_empty()).map(serde_json::from_str).collect::<Result<Vec<Row>, _>>()?;
        let raw = std::fs::read(f).unwrap_or_default();
        let feats = raw.as_chunks::<2>().0.iter().map(|p| f16::from_le_bytes(*p).to_f32()).collect();
        Ok(Ledger { rows, feats })
    }

    pub fn write(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut j = Vec::new();
        for r in &self.rows {
            serde_json::to_writer(&mut j, r)?;
            j.push(b'\n');
        }
        std::fs::write(dir.join("evidence.jsonl"), j)?;
        let mut f = std::io::BufWriter::new(std::fs::File::create(dir.join("evidence.f16"))?);
        for x in &self.feats {
            f.write_all(&f16::from_f32(*x).to_le_bytes())?;
        }
        Ok(())
    }

    /// Only the rows `keep` says, their features moved up to match
    pub fn retain(&mut self, keep: impl Fn(&Row) -> bool) {
        let w = width();
        let old = std::mem::take(self);
        for r in old.rows.iter().filter(|r| keep(r)) {
            let at = r.at as usize * w;
            self.push(r.clone(), &old.feats[at..at + w]);
        }
    }

    /// Rows for training `axis`: its own labels (not cheats, not rows on old encoders), and "*" rows that say it's clean
    pub fn for_axis(&self, axis: &str) -> Vec<usize> {
        (0..self.rows.len()).filter(|&i| {
            let r = &self.rows[i];
            r.source != "cheat" && r.source != "pair" && r.pin.is_none() && (r.axis == axis || (r.axis == "*" && !r.bad))
        }).collect()
    }

    /// A row's features replaced, as re-embedding does
    pub fn set_features(&mut self, i: usize, f: &[f32]) {
        let w = width();
        let at = self.rows[i].at as usize * w;
        self.feats[at..at + w].copy_from_slice(f);
    }
}

/// A window's features for a row, every modality, from a file's columns
pub fn features(c: &raea_sys_columns::Columns, a: f64, b: f64) -> Vec<f32> {
    Modality::ALL.iter().flat_map(|m| raea_sys_columns::window::features(c, a, b, *m)).collect()
}
