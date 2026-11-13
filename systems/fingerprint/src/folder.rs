//! A fingerprint's folder (FP2a): ~/.raea/fingerprints/NAME/ (a person's) or PROJECT/.raea/fingerprints/NAME/ (a
//! project's or team's), holding
//!
//! - fingerprint.ts: the axes, as `export default fingerprint({...})` from "raea/brief"; the person edits it
//! - fingerprint.json: what fingerprint.ts last evaluated to, so a host without a TS evaluator can still read it
//! - words.json and words.f16: the axes' words as vectors, and the background vocabulary (FP2e)
//! - heads.safetensors: each learned head's weights and scaler, its report in the metadata, the encoder pins
//! - evidence.jsonl and evidence.f16: the rows (evidence.rs)
//! - report.json: every head's held-out score, labels by source, cheats, gate state and limits (FP2d)

use crate::evidence::Ledger;
use crate::head::{Logistic, Scaler};
use crate::taxonomy::{Fingerprint, to_ts};
use crate::train::Trained;
use crate::words::Words;
use anyhow::{Context, Result, anyhow, bail};
use raea_sys_columns::cache::{f16s, f32s};
use raea_sys_columns::window::Modality;
use raea_sys_columns::{CLAP, SIG, TEXT};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Evaluates a .ts file's default export to its JSON (the CLI's spec host)
pub type Eval = fn(&Path) -> Result<String, String>;

/// Where a person's fingerprints live: $RAEA_FINGERPRINTS, else ~/.raea/fingerprints
pub fn root() -> PathBuf {
    raea_sys_columns::under("RAEA_FINGERPRINTS", ".raea/fingerprints")
}

pub struct Folder {
    pub dir: PathBuf,
}

/// words.json: what words.f16 holds, in order
#[derive(Serialize, Deserialize)]
struct WordsHead {
    axes: Vec<(String, Modality)>,
    sound_vocab: usize,
    picture_vocab: usize,
    #[serde(default)]
    text_vocab: usize,
    made_from: Vec<String>,
    /// the vocabulary's words in order, so a reader (the feed's fingerprint map) can name what a direction points at
    #[serde(default)]
    sound_words: Vec<String>,
    #[serde(default)]
    picture_words: Vec<String>,
}

impl Folder {
    /// A person's fingerprint by name, or a folder given as a path
    pub fn named(name: &str) -> Folder {
        let p = Path::new(name);
        Folder { dir: if p.is_absolute() || name.contains('/') { p.to_path_buf() } else { root().join(name) } }
    }

    pub fn exists(&self) -> bool {
        std::fs::metadata(self.ts()).is_ok_and(|m| m.is_file())
    }

    /// Its fingerprint.ts, the axes a person edits
    pub fn ts(&self) -> PathBuf {
        let mut p = self.dir.clone();
        p.push("fingerprint.ts");
        p
    }

    /// A new fingerprint's folder: its fingerprint.ts; refused when one is there already
    pub fn create(&self, f: &Fingerprint) -> Result<()> {
        if self.exists() {
            bail!("{} has a fingerprint already", self.dir.display());
        }
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.ts(), to_ts(f))?;
        std::fs::write(self.dir.join("fingerprint.json"), serde_json::to_vec_pretty(f)?)?;
        Ok(())
    }

    /// The fingerprint as fingerprint.ts says, through `eval`; without one, as fingerprint.ts last evaluated
    pub fn read(&self, eval: Option<Eval>) -> Result<Fingerprint> {
        if !self.exists() {
            bail!("no fingerprint at {} (`raea fingerprint new NAME` makes one)", self.dir.display());
        }
        let json = match eval {
            Some(e) => e(&self.ts()).map_err(|e| anyhow!("{e}"))?,
            None => std::fs::read_to_string(self.dir.join("fingerprint.json")).context("fingerprint.json")?,
        };
        let f: Fingerprint = serde_json::from_str(&json).with_context(|| format!("{}: not a fingerprint", self.ts().display()))?;
        if eval.is_some() {
            std::fs::write(self.dir.join("fingerprint.json"), serde_json::to_vec_pretty(&f)?)?;
        }
        Ok(f)
    }

    /// fingerprint.ts rewritten from the fingerprint (after its encoders are pinned)
    pub fn rewrite(&self, f: &Fingerprint) -> Result<()> {
        std::fs::write(self.ts(), to_ts(f))?;
        std::fs::write(self.dir.join("fingerprint.json"), serde_json::to_vec_pretty(f)?)?;
        Ok(())
    }

    pub fn words(&self) -> Option<Words> {
        let head: WordsHead = serde_json::from_slice(&std::fs::read(self.dir.join("words.json")).ok()?).ok()?;
        let v = f32s(&std::fs::read(self.dir.join("words.f16")).ok()?);
        let mut at = 0;
        let mut take = |n: usize| -> Option<Vec<f32>> {
            let r = v.get(at..at + n)?.to_vec();
            at += n;
            Some(r)
        };
        let mut axes = Vec::new();
        for (name, sp) in head.axes {
            let dim = match sp {
                Modality::Sound => CLAP,
                Modality::Text => TEXT,
                _ => SIG,
            };
            axes.push((name, sp, take(dim)?));
        }
        let sound_vocab = (0..head.sound_vocab).map(|_| take(CLAP)).collect::<Option<Vec<_>>>()?;
        let picture_vocab = (0..head.picture_vocab).map(|_| take(SIG)).collect::<Option<Vec<_>>>()?;
        let text_vocab = (0..head.text_vocab).map(|_| take(TEXT)).collect::<Option<Vec<_>>>()?;
        Some(Words { axes, sound_vocab, picture_vocab, text_vocab, made_from: head.made_from })
    }

    pub fn write_words(&self, w: &Words) -> Result<()> {
        let head = WordsHead {
            axes: w.axes.iter().map(|(n, sp, _)| (n.clone(), *sp)).collect(),
            sound_vocab: w.sound_vocab.len(),
            picture_vocab: w.picture_vocab.len(),
            text_vocab: w.text_vocab.len(),
            made_from: w.made_from.clone(),
            sound_words: crate::taxonomy::SOUND_VOCAB.iter().take(w.sound_vocab.len()).map(|s| s.to_string()).collect(),
            picture_words: crate::taxonomy::PICTURE_VOCAB.iter().take(w.picture_vocab.len()).map(|s| s.to_string()).collect(),
        };
        let v: Vec<f32> = w.axes.iter().map(|a| &a.2).chain(&w.sound_vocab).chain(&w.picture_vocab).chain(&w.text_vocab).flatten().copied().collect();
        std::fs::write(self.dir.join("words.json"), serde_json::to_vec_pretty(&head)?)?;
        std::fs::write(self.dir.join("words.f16"), f16s(&v))?;
        Ok(())
    }

    /// The evidence: one ledger, or a project's or workspace's every person's (share.rs, FP8a)
    pub fn ledger(&self) -> Result<Ledger> {
        if crate::share::shared(&self.dir) { crate::share::read(&self.dir) } else { Ledger::read(&self.dir) }
    }

    pub fn write_ledger(&self, l: &Ledger) -> Result<()> {
        if crate::share::shared(&self.dir) { crate::share::write(&self.dir, l) } else { l.write(&self.dir) }
    }

    /// The trained heads and the encoders they were trained on; None before the first training
    pub fn trained(&self) -> Result<Option<(Trained, BTreeMap<String, String>)>> {
        let p = self.dir.join("heads.safetensors");
        if !p.is_file() {
            return Ok(None);
        }
        let (meta, tensors) = read_safetensors(&p)?;
        let mut t: Trained = serde_json::from_str(meta.get("trained").map(String::as_str).unwrap_or("{}"))?;
        let pins: BTreeMap<String, String> = serde_json::from_str(meta.get("encoders").map(String::as_str).unwrap_or("{}"))?;
        let head = |k: &str| -> Option<Logistic> {
            let get = |part: &str| tensors.get(&format!("{k}/{part}")).cloned();
            Some(Logistic { scaler: Scaler { mean: get("mean")?, std: get("std")? }, w: get("w")?, b: *get("b")?.first()? })
        };
        for h in &mut t.heads {
            h.head = head(&h.axis);
        }
        t.weighting = head("*weighting");
        Ok(Some((t, pins)))
    }

    pub fn write_trained(&self, t: &Trained, pins: &BTreeMap<String, String>) -> Result<()> {
        fn put<'a>(tensors: &mut Vec<(String, &'a [f32])>, k: &str, l: &'a Logistic) {
            let b = std::slice::from_ref(&l.b);
            for (part, v) in [("mean", &l.scaler.mean[..]), ("std", &l.scaler.std[..]), ("w", &l.w[..]), ("b", b)] {
                tensors.push((format!("{k}/{part}"), v));
            }
        }
        let mut tensors: Vec<(String, &[f32])> = Vec::new();
        for h in &t.heads {
            if let Some(l) = &h.head {
                put(&mut tensors, &h.axis, l);
            }
        }
        if let Some(l) = &t.weighting {
            put(&mut tensors, "*weighting", l);
        }
        let meta = BTreeMap::from([("trained".to_string(), serde_json::to_string(t)?), ("encoders".to_string(), serde_json::to_string(pins)?)]);
        write_safetensors(&self.dir.join("heads.safetensors"), &meta, &tensors)
    }

    pub fn write_report(&self, report: &serde_json::Value) -> Result<()> {
        std::fs::write(self.dir.join("report.json"), serde_json::to_vec_pretty(report)?)?;
        Ok(())
    }

    pub fn report(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&std::fs::read(self.dir.join("report.json")).ok()?).ok()
    }
}

/// safetensors: an 8-byte little-endian header length, the JSON header (padded with spaces to 8 bytes), then the data
fn write_safetensors(p: &Path, meta: &BTreeMap<String, String>, tensors: &[(String, &[f32])]) -> Result<()> {
    let mut head = serde_json::Map::new();
    head.insert("__metadata__".into(), serde_json::to_value(meta)?);
    let mut at = 0usize;
    for (k, v) in tensors {
        let end = at + 4 * v.len();
        head.insert(k.clone(), serde_json::json!({"dtype": "F32", "shape": [v.len()], "data_offsets": [at, end]}));
        at = end;
    }
    let mut h = serde_json::to_vec(&head)?;
    while h.len() % 8 != 0 {
        h.push(b' ');
    }
    let mut out = (h.len() as u64).to_le_bytes().to_vec();
    out.extend(h);
    for (_, v) in tensors {
        out.extend(v.iter().flat_map(|x| x.to_le_bytes()));
    }
    let tmp = p.with_extension("part");
    std::fs::write(&tmp, out)?;
    std::fs::rename(tmp, p)?;
    Ok(())
}

type Tensors = BTreeMap<String, Vec<f32>>;

fn read_safetensors(p: &Path) -> Result<(BTreeMap<String, String>, Tensors)> {
    let b = std::fs::read(p)?;
    let n = u64::from_le_bytes(b.get(..8).ok_or_else(|| anyhow!("{}: too short", p.display()))?.try_into()?) as usize;
    let head: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(b.get(8..8 + n).ok_or_else(|| anyhow!("{}: header past the end", p.display()))?)?;
    let data = &b[8 + n..];
    let mut meta = BTreeMap::new();
    let mut tensors = BTreeMap::new();
    for (k, v) in head {
        if k == "__metadata__" {
            meta = serde_json::from_value(v)?;
            continue;
        }
        let o = v["data_offsets"].as_array().ok_or_else(|| anyhow!("{k}: no offsets"))?;
        let (a, e) = (o[0].as_u64().unwrap_or(0) as usize, o[1].as_u64().unwrap_or(0) as usize);
        let raw = data.get(a..e).ok_or_else(|| anyhow!("{k}: data past the end"))?;
        tensors.insert(k, raw.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect());
    }
    Ok((meta, tensors))
}
