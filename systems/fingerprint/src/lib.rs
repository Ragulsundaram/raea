//! Fingerprint (docs/fingerprint.feature FP2-FP4, docs/fingerprint.md): a person's or a team's sense of good work as
//! a taxonomy of axes, each with a small head over the frozen encoders' columns (raea-sys-columns), trained on the
//! device in milliseconds from evidence rows, gated until it has earned it, and scored per moment.
//!
//! - taxonomy.rs: the axes and raea's base taxonomy (FP2b)
//! - folder.rs: the fingerprint's folder (FP2a)
//! - evidence.rs: the rows (FP2c)
//! - head.rs, train.rs: the heads, held-out scores, bands, cheats and the gate (FP2c, FP2d)
//! - words.rs: word heads from day one (FP2e)
//! - score.rs, sheet.rs: scoring a piece, its summary, timeline and sheet (FP3a, FP3c)

pub mod evidence;
pub mod folder;
pub mod head;
pub mod score;
pub mod sheet;
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

/// Every head retrained from the ledger, written with the report; the bootstrap's measures are kept from the last
/// report unless a new bootstrap gives its own
pub fn retrain(dir: &Folder, fp: &Fingerprint, w: &Words, l: &Ledger, bootstrap: Option<serde_json::Value>) -> Result<Trained> {
    let t = train::train(fp, l, w);
    dir.write_trained(&t, &fp.encoders)?;
    let kept = bootstrap.or_else(|| dir.report().and_then(|r| r.get("bootstrap").cloned()));
    dir.write_report(&report(fp, &t, l, kept))?;
    Ok(t)
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
