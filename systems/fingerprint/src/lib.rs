//! Fingerprint (docs/fingerprint.feature FP2-FP4, docs/fingerprint.md): a person's or a team's sense of good work as
//! a taxonomy of axes, each with a small head over the frozen encoders' columns (raea-sys-columns), trained on the
//! device in milliseconds from evidence rows, and gated until it has earned it. Scoring a piece with one, and the
//! tools around it, are raea-sys-fingerprint-use.
//!
//! - taxonomy.rs: the axes and raea's base taxonomy (FP2b)
//! - folder.rs: the fingerprint's folder (FP2a)
//! - evidence.rs: the rows (FP2c)
//! - head.rs, train.rs: the heads, held-out scores, bands, cheats and the gate (FP2c, FP2d)
//! - words.rs: word heads from day one (FP2e)
//! - inherit.rs: a fingerprint that extends another, its evidence and axes (FP14b)
//! - pairs.rs: Bradley-Terry fits on picks, hand edits and version pairs (FP6b, FP6c)
//! - share.rs: a project's or workspace's fingerprint, its evidence a folder a person (FP8a)

pub mod evidence;
pub mod folder;
pub mod head;
pub mod inherit;
pub mod pairs;
pub mod share;
pub mod taxonomy;
pub mod train;
pub mod words;

use anyhow::Result;
use evidence::Ledger;
use folder::Folder;
use serde_json::json;
use std::collections::BTreeMap;
use taxonomy::Fingerprint;
use train::{GATE_AUC, GATE_CHEAT, GATE_LABELS, GATE_PROJECTS, Trained};
use words::Words;

/// What no head here can judge, whatever its evidence (earshot's LIMITS.md)
pub const LIMITS: [&str; 5] = [
    "whether the piece works as a whole, or for its audience: heads judge 2 s moments",
    "story, pace across shots and the cut's rhythm: no axis reads more than a window",
    "where in the frame a defect is, until marks teach patch heads (FP12)",
    "whether a stylised look or a held shot was meant: a word head reads it as the defect",
    "taste axes with only words: always advice, never a gate",
];

/// Every head retrained from the ledger (with its parents' evidence when it extends one, FP14b), written with the
/// report; the bootstrap's measures are kept from the last report unless a new bootstrap gives its own
pub fn retrain(dir: &Folder, fp: &Fingerprint, w: &Words, l: &Ledger, bootstrap: Option<serde_json::Value>) -> Result<Trained> {
    let parents = inherit::parents(fp, None);
    let over = inherit::overrides(fp, &parents);
    let fp = &inherit::axes(fp, &parents);
    let (all, own) = inherit::pooled(l, &parents, &over);
    let t = train::train(fp, &all, w);
    dir.write_trained(&t, &fp.encoders)?;
    let kept = bootstrap.or_else(|| Some(dir.report()?.get("bootstrap")?.clone()));
    let mut r = report(fp, &t, &all, kept);
    if !parents.is_empty() {
        r["inherits"] = json!({
            "from": parents.iter().map(|(_, p)| p.name.as_str()).collect::<Vec<_>>(),
            "own_rows": own,
            "inherited_rows": all.rows.len() - own,
            "own_weight": inherit::OWN,
            "overrides": over,
            "measured": inherit::measure(fp, &all, own),
        });
    }
    dir.write_report(&r)?;
    Ok(t)
}

/// An axis this many labels deep and still under the held-out bar is stuck (FP10a)
pub const STUCK_LABELS: usize = 300;

/// The axes stuck under their bar with STUCK_LABELS or more labels on their own columns (docs/fingerprint.feature
/// FP10a): only these may try an adapter on SigLIP 2 or another encoder, kept only if it lifts held out by 0.05
pub fn stuck(t: &Trained) -> Vec<(String, usize, f64)> {
    t.heads
        .iter()
        .filter_map(|h| {
            // a cheat is a test of the head, not a label of its axis
            let n: usize = h.labels.iter().filter(|(s, _)| s.as_str() != "cheat").map(|(_, [a, b])| a + b).sum();
            let auc = h.held_out?;
            (n >= STUCK_LABELS && auc < GATE_AUC).then(|| (h.axis.clone(), n, auc))
        })
        .collect()
}

/// report.json (FP2d): every head's held-out score, labels by source, cheat catch rates, gate state and limits
pub fn report(fp: &Fingerprint, t: &Trained, l: &Ledger, bootstrap: Option<serde_json::Value>) -> serde_json::Value {
    let mut by_source: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &l.rows {
        *by_source.entry(r.source.as_str()).or_default() += 1;
    }
    let made = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    json!({
        "fingerprint": fp.name,
        "who": fp.who,
        "made_unix": made,
        "encoders": fp.encoders,
        "gate": {"labels_each": GATE_LABELS, "projects": GATE_PROJECTS, "held_out_auc": GATE_AUC, "cheat_caught": GATE_CHEAT},
        "evidence": {"rows": l.rows.len(), "by_source": by_source},
        "heads": t.heads,
        "gating": t.heads.iter().filter(|h| h.state == "gating").map(|h| h.axis.as_str()).collect::<Vec<_>>(),
        "weighting": t.weighting.is_some(),
        "stuck": stuck(t).into_iter().map(|(axis, labels, held_out)| json!({"axis": axis, "labels": labels, "held_out": held_out})).collect::<Vec<_>>(),
        "bootstrap": bootstrap,
        "cannot_judge": LIMITS,
    })
}

/// report.json read back as lines for a person: each head's state, score, labels and cheats
pub fn report_text(r: &serde_json::Value) -> String {
    let mut out = format!("fingerprint {}: {} evidence rows\n", r["fingerprint"].as_str().unwrap_or("?"), r["evidence"]["rows"]);
    let f = |v: &serde_json::Value| v.as_f64().map_or("-".to_string(), |x| format!("{x:.2}"));
    for h in r["heads"].as_array().into_iter().flatten() {
        let labels: usize = h["labels"].as_object().map_or(0, |m| m.values().filter_map(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_u64()).sum::<u64>() as usize).sum());
        let cheats: Vec<String> = h["cheats"].as_object().map_or(Vec::new(), |m| m.iter().map(|(k, v)| format!("{k} {}", f(v))).collect());
        out += &format!(
            "  {:16} {:7} held out {:>4}  words {:>4}  labels {:>5}  {}{}\n",
            h["axis"].as_str().unwrap_or(""),
            h["state"].as_str().unwrap_or(""),
            f(&h["held_out"]),
            f(&h["words"]),
            labels,
            if cheats.is_empty() { String::new() } else { format!("cheats: {}; ", cheats.join(", ")) },
            h["why"].as_str().unwrap_or("")
        );
    }
    let stuck = r["stuck"].as_array().cloned().unwrap_or_default();
    out += &match stuck.is_empty() {
        true => format!("stuck: none (an axis {STUCK_LABELS} labels deep under held out {GATE_AUC} may try an adapter or another encoder)\n"),
        false => format!(
            "stuck, so each may try an adapter on SigLIP 2 or another encoder, kept only at 0.05 better held out: {}\n",
            stuck.iter().map(|x| format!("{} ({} labels, held out {})", x["axis"].as_str().unwrap_or(""), x["labels"], f(&x["held_out"]))).collect::<Vec<_>>().join(", ")
        ),
    };
    if let Some(b) = r["bootstrap"].as_object() {
        out += "bootstrap, held out by source (AUC at k = 4 / 16 / 32, FP0b's k = 32):\n";
        for c in b.get("curves").and_then(|c| c.as_array()).into_iter().flatten() {
            let k = |n: &str| f(&c["k"][n]);
            let holds = match c["holds"].as_bool() {
                Some(true) => "holds",
                Some(false) => "BELOW",
                None => "",
            };
            out += &format!("  {:8} {:8} {} / {} / {}   {} {holds}\n", c["axis"].as_str().unwrap_or(""), c["modality"].as_str().unwrap_or(""), k("4"), k("16"), k("32"), f(&c["fp0b"]));
        }
        for w in b.get("words").and_then(|c| c.as_array()).into_iter().flatten() {
            out += &format!("  words {:8} AUC {}  caught {}  cheats {}\n", w["axis"].as_str().unwrap_or(""), f(&w["auc"]), f(&w["caught"]), w["cheats"]);
        }
    }
    out += "can't judge:\n";
    for l in r["cannot_judge"].as_array().into_iter().flatten() {
        out += &format!("  {}\n", l.as_str().unwrap_or(""));
    }
    out
}

#[cfg(test)]
mod tests;
