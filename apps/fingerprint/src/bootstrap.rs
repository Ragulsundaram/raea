//! The bootstrap (FP4a, FP4b): known defects put into windows of the person's own real work, so the mechanical axes
//! learn from labels by construction, and the cheats each must still catch. Each source gives `per` 2 s windows
//! from 0.5 s to its end less 2.5 s (FP0b's protocol); each window gives a clean row ("*"), a row per picture and
//! sound defect, and a row per cheat. Only what the damage changed is embedded again: a freeze reuses the first
//! frame's vector, a flash re-embeds one frame, a sound defect leaves the picture alone.
//!
//! Then it measures what FP0b measured, in Rust: each axis's AUC from k = 4, 16 and 32 labelled windows a class, held
//! out by source (5 folds x 5 draws, the scaler fit on every plain row of the training sources), on its own
//! modality and on the one FP0b used; and each word head's AUC and catch rate with no labels (FP0g).

use raea_sys_fingerprint::evidence::{Ledger, Row, features, slice};
use raea_sys_fingerprint::head::{Logistic, Scaler, auc, quantile, share_over};
use raea_sys_fingerprint::taxonomy::Fingerprint;
use raea_sys_fingerprint::train::{C, folds};
use raea_sys_fingerprint::words::Words;
use anyhow::Result;
use raea_sys_columns::damage::{self, CHEATS, Noise, PICTURE, SOUND};
use raea_sys_columns::media::{self, Rgb};
use raea_sys_columns::window::Modality;
use raea_sys_columns::{Columns, Encoders, FPS};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The window, seconds
pub const WIN: f64 = 2.0;
/// Sources shorter than this are skipped (FP0b's)
pub const SHORTEST: f64 = 6.0;
pub const KS: [usize; 3] = [4, 16, 32];

/// FP0b's AUC at k = 32 for each axis, on the modality it was measured on (docs/fingerprint.md FP0b, FP0g)
pub const FP0B: [(&str, Modality, f64); 11] = [
    ("blur", Modality::Picture, 1.00),
    ("dark", Modality::Picture, 0.95),
    ("glow", Modality::Picture, 0.80),
    ("flash", Modality::Change, 0.97),
    ("freeze", Modality::Change, 1.00),
    ("noise", Modality::Events, 1.00),
    ("dropout", Modality::Events, 0.97),
    ("clip", Modality::Events, 0.98),
    ("noise", Modality::Sound, 1.00),
    ("dropout", Modality::Sound, 0.99),
    ("clip", Modality::Sound, 1.00),
];

#[derive(Debug, Default, Serialize)]
pub struct Made {
    pub sources: usize,
    pub windows: usize,
    pub rows: usize,
    /// windows too quiet for a gap or clipping to change them, which get only the noise damage
    pub quiet: usize,
    pub skipped: Vec<String>,
}

/// The rows a bootstrap makes, which the next bootstrap replaces
fn made_here(r: &Row) -> bool {
    matches!(r.source.as_str(), "damage" | "cheat")
}

/// A plain damage row on `axis` ("*" for the clean windows)
fn damage_on(r: &Row, axis: &str) -> bool {
    r.source == "damage" && r.axis == axis
}

/// The plain damage rows on `axis`
fn damage_rows(l: &Ledger, axis: &str) -> Vec<usize> {
    (0..l.rows.len()).filter(|&i| damage_on(&l.rows[i], axis)).collect()
}

/// Each defect of a sense as (kind, axis, source): the plain ones, then their disguises
fn damages(sense: &[&'static str]) -> Vec<(&'static str, &'static str, &'static str)> {
    sense.iter().map(|k| (*k, *k, "damage")).chain(CHEATS.iter().filter(|c| sense.contains(&c.1)).map(|c| (c.0, c.1, "cheat"))).collect()
}

/// Below this a window is near silence: a gap or clipping changes nothing, and a row saying it does teaches noise
const QUIET_DB: f32 = -45.0;

/// The window's loudness in dB over all of it
fn loudness(pcm: &[f32]) -> f32 {
    if pcm.is_empty() {
        return -100.0;
    }
    (10.0 * (pcm.iter().map(|x| x * x).sum::<f32>() / pcm.len() as f32).max(1e-10).log10()).max(-100.0)
}

fn zeros_to(mut v: Vec<f32>, n: usize) -> Vec<f32> {
    v.resize(n, 0.0);
    v
}

/// The picture columns of damaged frames, embedding only frames the damage changed
fn picture_of(enc: &mut Encoders, clean: &[Rgb], sig: &[f32], look: &[f32], d: &[Rgb]) -> Result<(Vec<f32>, Vec<f32>)> {
    let (sd, ld) = (sig.len() / clean.len().max(1), look.len() / clean.len().max(1));
    let mut fresh: Vec<Rgb> = Vec::new();
    let mut from: Vec<Result<usize, usize>> = Vec::new();
    for f in d {
        match clean.iter().position(|c| c == f) {
            Some(j) => from.push(Ok(j)),
            None => {
                fresh.push(f.clone());
                from.push(Err(fresh.len() - 1));
            }
        }
    }
    let (fs, fl) = if fresh.is_empty() { (Vec::new(), Vec::new()) } else { enc.picture(&fresh)? };
    let (mut s, mut l) = (Vec::with_capacity(sig.len()), Vec::with_capacity(look.len()));
    for f in from {
        let (ss, ll, j) = match f {
            Ok(j) => (sig, look, j),
            Err(k) => (&fs[..], &fl[..], k),
        };
        s.extend_from_slice(&ss[j * sd..(j + 1) * sd]);
        l.extend_from_slice(&ll[j * ld..(j + 1) * ld]);
    }
    Ok((s, l))
}

/// The bootstrap's rows for `files` into the ledger, replacing any it made before. `say` hears each source's progress.
pub fn rows(enc: &mut Encoders, who: &str, files: &[PathBuf], per: usize, l: &mut Ledger, say: &mut dyn FnMut(String)) -> Result<Made> {
    l.retain(|r| !made_here(r));
    let mut made = Made::default();
    let mut noise = Noise::new(0);
    for (n, file) in files.iter().enumerate() {
        let dur = media::duration(file).unwrap_or(0.0);
        if dur < SHORTEST {
            made.skipped.push(format!("{} ({dur:.1} s, under {SHORTEST} s)", file.display()));
            continue;
        }
        let sha = media::sha256(file)?;
        let frames = media::frames(file, FPS, 0.0, dur)?;
        let (p16, p48) = (media::sound(file, 16_000)?, media::sound(file, 48_000)?);
        made.sources += 1;
        for w in 0..per {
            let t = 0.5 + (dur - 3.0) * w as f64 / (per - 1).max(1) as f64;
            let i0 = (t * FPS).round() as usize;
            let Some(fr) = frames.get(i0..i0 + (WIN * FPS) as usize) else { continue };
            let cut = |p: &[f32], rate: f64| zeros_to(p.get((t * rate) as usize..).unwrap_or(&[]).iter().take((WIN * rate) as usize).copied().collect(), (WIN * rate) as usize);
            let (w16, w48) = (cut(&p16, 16_000.0), cut(&p48, 48_000.0));
            let (sig, look) = enc.picture(fr)?;
            let (clap, yam) = enc.sound(&w16, &w48)?;
            let clean = Columns { media: sha.clone(), dur: WIN, sig, look, clap, yam, level: media::levels(&w16, 16_000), text: Vec::new() };
            let push = |l: &mut Ledger, c: &Columns, axis: &str, kind: &str, source: &str| {
                let r = Row {
                    media: sha.clone(),
                    a: t,
                    b: t + WIN,
                    axis: axis.into(),
                    bad: axis != "*",
                    weight: 1.0,
                    who: who.into(),
                    source: source.into(),
                    project: sha[..12].to_string(),
                    kind: Some(kind.into()),
                    region: None,
                    words: None,
                    at: 0,
                    pin: None,
                    band: None,
                };
                l.push(r, &features(c, 0.0, WIN));
            };
            push(l, &clean, "*", "clean", "damage");
            for (kind, axis, source) in damages(&PICTURE) {
                let (sig, look) = picture_of(enc, fr, &clean.sig, &clean.look, &damage::picture(fr, kind))?;
                push(l, &Columns { sig, look, ..clean.clone() }, axis, kind, source);
            }
            let quiet = loudness(&w16) < QUIET_DB;
            made.quiet += quiet as usize;
            for (kind, axis, source) in damages(&SOUND).into_iter().filter(|s| !quiet || s.1 == "noise") {
                let (d16, d48) = (damage::sound(&w16, kind, 16_000, &mut noise), damage::sound(&w48, kind, 48_000, &mut noise));
                let (clap, yam) = enc.sound(&d16, &d48)?;
                push(l, &Columns { clap, yam, level: media::levels(&d16, 16_000), ..clean.clone() }, axis, kind, source);
            }
            made.windows += 1;
        }
        say(format!("source {}/{} {}: {} windows so far", n + 1, files.len(), file.display(), made.windows));
    }
    made.rows = l.rows.iter().filter(|r| made_here(r)).count();
    Ok(made)
}

#[derive(Debug, Serialize)]
pub struct Curve {
    pub axis: String,
    pub modality: Modality,
    /// AUC at each k
    pub k: BTreeMap<usize, f64>,
    /// FP0b's at k = 32 on this modality, and whether this run came within 0.03 of it or beat it
    pub fp0b: Option<f64>,
    pub holds: Option<bool>,
}

fn sample(rng: &mut fastrand::Rng, from: &[usize], k: usize) -> Vec<usize> {
    let mut v = from.to_vec();
    rng.shuffle(&mut v);
    v.truncate(k);
    v
}

/// FP0b's few-shot curve for one axis on one modality, from the bootstrap's rows
pub fn curve(l: &Ledger, axis: &str, m: Modality, rng: &mut fastrand::Rng) -> Curve {
    let s = slice(m);
    let row = |i: usize| &l.features(i)[s.clone()];
    let plain = |i: &usize| l.rows[*i].source == "damage";
    let (clean, bad) = (damage_rows(l, "*"), damage_rows(l, axis));
    let projects = l.projects(&clean);
    let mut k_auc = BTreeMap::new();
    for k in KS {
        let mut aucs = Vec::new();
        for fold in folds(&projects) {
            let held = |i: &usize| fold.contains(&l.rows[*i].project);
            let base: Vec<&[f32]> = (0..l.rows.len()).filter(|i| plain(i) && !held(i)).map(row).collect();
            let sc = Scaler::fit(&base);
            let (tc, tb): (Vec<usize>, Vec<usize>) = (clean.iter().copied().filter(|i| !held(i)).collect(), bad.iter().copied().filter(|i| !held(i)).collect());
            if tc.len() < k || tb.len() < k {
                continue;
            }
            for _ in 0..5 {
                let (a, b) = (sample(rng, &tc, k), sample(rng, &tb, k));
                let x: Vec<&[f32]> = a.iter().chain(&b).map(|&i| row(i)).collect();
                let y: Vec<bool> = (0..2 * k).map(|i| i >= k).collect();
                let f = Logistic::fit(sc.clone(), &x, &y, None, C, true);
                let score = |v: &Vec<usize>| -> Vec<f32> { v.iter().filter(|i| held(i)).map(|&i| f.score(row(i))).collect() };
                aucs.push(auc(&score(&bad), &score(&clean)));
            }
        }
        let ok: Vec<f64> = aucs.into_iter().filter(|a| a.is_finite()).collect();
        if !ok.is_empty() {
            k_auc.insert(k, ok.iter().sum::<f64>() / ok.len() as f64);
        }
    }
    let fp0b = FP0B.iter().find(|(a, mm, _)| *a == axis && *mm == m).map(|x| x.2);
    let holds = fp0b.zip(k_auc.get(&32)).map(|(want, got)| *got >= want - 0.03);
    Curve { axis: axis.into(), modality: m, k: k_auc, fp0b, holds }
}

/// Every mechanical axis's curve on its own modality and on FP0b's
pub fn curves(fp: &Fingerprint, l: &Ledger) -> Vec<Curve> {
    let mut rng = fastrand::Rng::with_seed(1);
    let mut out = Vec::new();
    for axis in PICTURE.iter().chain(&SOUND) {
        let own = fp.axis(axis).map(|a| a.modality);
        let mut ms: Vec<Modality> = own.into_iter().chain(FP0B.iter().filter(|x| x.0 == *axis).map(|x| x.1)).collect();
        ms.dedup();
        for m in ms.into_iter().collect::<BTreeSet<_>>() {
            out.push(curve(l, axis, m, &mut rng));
        }
    }
    out
}

#[derive(Debug, Serialize)]
pub struct WordScore {
    pub axis: String,
    /// AUC, damaged against clean windows, with no labels
    pub auc: f64,
    /// the share of damaged windows over the score that flags 10% of clean ones
    pub caught: f64,
    /// each cheat's share over that same score
    pub cheats: BTreeMap<String, f64>,
}

/// Each word head on the bootstrap's windows (FP0g)
pub fn word_scores(l: &Ledger, words: &Words) -> Vec<WordScore> {
    let z = |i: usize, axis: &str| words.z(axis, l.features(i));
    let of = |axis: &str, pick: &dyn Fn(&Row) -> bool| -> Vec<f32> { l.rows.iter().enumerate().filter(|(_, r)| pick(r)).filter_map(|(i, _)| z(i, axis)).collect() };
    let mut out = Vec::new();
    for axis in PICTURE.iter().chain(&SOUND) {
        if !words.axes.iter().any(|w| w.0 == *axis) {
            continue;
        }
        let clean = of(axis, &|r| damage_on(r, "*"));
        let bad = of(axis, &|r| damage_on(r, axis));
        let thr = quantile(&clean, 0.9);
        let share = |v: &[f32]| share_over(v, thr);
        let cheats = CHEATS.iter().filter(|c| c.1 == *axis).map(|c| (c.0.to_string(), share(&of(axis, &|r| r.source == "cheat" && r.kind.as_deref() == Some(c.0))))).collect();
        out.push(WordScore { axis: axis.to_string(), auc: auc(&bad, &clean), caught: share(&bad), cheats });
    }
    out
}
