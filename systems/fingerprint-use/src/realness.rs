//! Realness, measured before it ships (docs/fingerprint.feature FP2f): a head on a sound's CLAP and YAMNet means,
//! trained from labels by construction, raea's own generators' sounds (SAO's foley, TokenSynth's notes, Fish's
//! narration) against recordings of the same kinds of sound, and held out two ways as earshot holds its own out: by
//! generator (trained on the other two generators and their recordings, tested on the third and its) and by category
//! (each category left out in turn). It ships as an axis only at SHIPS or better held out by generator; it's a taste
//! check ("real material only"), never a claim that a sound is or isn't AI-made.

use raea_sys_fingerprint::head::{Logistic, Scaler, auc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Held out by generator, the least of the three, for the axis to ship
pub const SHIPS: f64 = 0.85;
/// The head's C (as the axis heads')
const C: f64 = 1.0;

/// A sound with its label by construction: made by a generator (`by`), or a recording (`by` "recorded")
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub file: PathBuf,
    pub by: String,
    /// what kind of sound: foley, instrument or speech; each generator makes one kind
    pub kind: String,
    pub category: String,
}

impl Item {
    fn made(&self) -> bool {
        self.by != "recorded"
    }
}

/// One held-out fold's AUC (made scoring above recorded), with its counts
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Held {
    pub out: String,
    pub auc: Option<f64>,
    pub made: usize,
    pub recorded: usize,
}

/// Fit on the rows `train` picks, score the rows `test` picks: the AUC of made over recorded
fn fold(x: &[(Item, Vec<f32>)], train: impl Fn(&Item) -> bool, test: impl Fn(&Item) -> bool, out: String) -> Held {
    let tr: Vec<&(Item, Vec<f32>)> = x.iter().filter(|r| train(&r.0)).collect();
    let rows: Vec<&[f32]> = tr.iter().map(|r| r.1.as_slice()).collect();
    let y: Vec<bool> = tr.iter().map(|r| r.0.made()).collect();
    let (mut made, mut recorded) = (Vec::new(), Vec::new());
    let both = y.iter().any(|v| *v) && y.iter().any(|v| !*v);
    let head = both.then(|| Logistic::fit(Scaler::fit(&rows), &rows, &y, None, C, true));
    for r in x.iter().filter(|r| test(&r.0)) {
        let s = head.as_ref().map_or(0.0, |h| h.score(&r.1));
        if r.0.made() { made.push(s) } else { recorded.push(s) }
    }
    let ok = head.is_some() && !made.is_empty() && !recorded.is_empty();
    Held { out, auc: ok.then(|| auc(&made, &recorded)), made: made.len(), recorded: recorded.len() }
}

/// Held out by generator: each generator and the recordings of its kind left out in turn
pub fn by_generator(x: &[(Item, Vec<f32>)]) -> Vec<Held> {
    let mut gens: Vec<(String, String)> = x.iter().filter(|r| r.0.made()).map(|r| (r.0.by.clone(), r.0.kind.clone())).collect();
    gens.sort();
    gens.dedup();
    gens.into_iter().map(|(g, k)| fold(x, |i| i.kind != k, |i| i.kind == k, g)).collect()
}

/// Held out by category: each category (rain, piano, speech...) left out in turn, made and recorded alike
pub fn by_category(x: &[(Item, Vec<f32>)]) -> Vec<Held> {
    let mut cats: Vec<String> = x.iter().map(|r| r.0.category.clone()).collect();
    cats.sort();
    cats.dedup();
    cats.into_iter().map(|c| fold(x, |i| i.category != c, |i| i.category == c, c.clone())).collect()
}

/// Whether the axis ships: every generator held out at SHIPS or better
pub fn ships(held: &[Held]) -> bool {
    !held.is_empty() && held.iter().all(|h| h.auc.is_some_and(|a| a >= SHIPS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(by: &str, kind: &str, category: &str) -> Item {
        Item { file: PathBuf::new(), by: by.into(), kind: kind.into(), category: category.into() }
    }

    #[test]
    fn held_out_by_generator_trains_on_the_others_only() {
        // made sound sits high on the first feature, recorded low, in every kind: a head trained on two kinds finds
        // the third's; a feature only one kind uses (the second) says nothing held out
        let mut x = Vec::new();
        for (g, k) in [("sao", "foley"), ("tokensynth", "instrument"), ("fish", "speech")] {
            for i in 0..12 {
                let j = i as f32 * 0.01;
                x.push((item(g, k, k), vec![1.0 + j, if k == "speech" { 5.0 } else { 0.0 }]));
                x.push((item("recorded", k, k), vec![-1.0 - j, if k == "speech" { 5.0 } else { 0.0 }]));
            }
        }
        let g = by_generator(&x);
        assert_eq!(g.iter().map(|h| (h.out.as_str(), h.made, h.recorded)).collect::<Vec<_>>(), [("fish", 12, 12), ("sao", 12, 12), ("tokensynth", 12, 12)]);
        assert!(ships(&g), "{g:?}");
        // a generator whose sound is no different from its recordings doesn't ship
        for r in x.iter_mut().filter(|r| r.0.kind == "speech") {
            r.1[0] = 0.0;
        }
        assert!(!ships(&by_generator(&x)));
        assert_eq!(by_category(&x).len(), 3);
    }
}

#[cfg(test)]
mod real {
    use super::*;
    use raea_sys_columns::window::{Modality, features};

    /// `RAEA_REALNESS=DIR cargo test -p raea-sys-fingerprint-use realness_on -- --ignored --nocapture`: DIR/made/made.jsonl
    /// and DIR/real/real.jsonl (each a file, its generator or source and category), measured; DIR/report.json keeps it
    #[test]
    #[ignore]
    fn realness_on_this_machine() {
        let dir = PathBuf::from(std::env::var("RAEA_REALNESS").expect("RAEA_REALNESS"));
        let kind = |c: &str| match c {
            "speech" => "speech",
            "brass" | "flute" | "guitar" | "keyboard" | "mallet" | "reed" | "string" | "bass" | "organ" => "instrument",
            _ => "foley",
        };
        let mut items = Vec::new();
        for (f, made) in [("made/made.jsonl", true), ("real/real.jsonl", false)] {
            for l in std::fs::read_to_string(dir.join(f)).unwrap().lines() {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                let c = v["category"].as_str().unwrap().to_string();
                let by = if made { v["by"].as_str().unwrap().to_string() } else { "recorded".into() };
                items.push(Item { file: v["file"].as_str().unwrap().into(), by, kind: kind(&c).into(), category: c });
            }
        }
        // a category with no recordings (TokenSynth's bass and organ) can't be held out against anything
        let recorded: Vec<String> = items.iter().filter(|i| !i.made()).map(|i| i.category.clone()).collect();
        items.retain(|i| recorded.contains(&i.category));
        let mut enc = raea_sys_columns::Encoders::load().unwrap();
        let x: Vec<(Item, Vec<f32>)> = items
            .into_iter()
            .map(|i| {
                let c = enc.columns(&i.file).unwrap();
                let f = [Modality::Sound, Modality::Events].iter().flat_map(|m| features(&c, 0.0, c.dur, *m)).collect();
                (i, f)
            })
            .collect();
        let (g, c) = (by_generator(&x), by_category(&x));
        let r = serde_json::json!({"by_generator": g, "by_category": c, "ships": ships(&g), "bar": SHIPS, "items": x.len()});
        std::fs::write(dir.join("report.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        eprintln!("{}", serde_json::to_string_pretty(&r).unwrap());
    }
}
