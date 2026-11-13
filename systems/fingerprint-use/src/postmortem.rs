//! A postmortem (docs/fingerprint.feature FP7a): when a project closes, or on `raea fingerprint postmortem PROJECT`,
//! every version of every piece is scored, and the moments where the fingerprint disagrees with the person's own
//! labels, then those where its heads are least sure, become at most ten yes/no questions in the app, each with its
//! moment playing. Each answer is an evidence row (source "postmortem"); `postmortem --tally` adds them and the report
//! says how each head's held-out score moved.

use crate::score::Scored;
use anyhow::{Result, anyhow};
use raea_sys_fingerprint::evidence::Ledger;
use raea_sys_fingerprint::taxonomy::Fingerprint;
use raea_sys_fingerprint::train::Trained;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// At most this many questions a postmortem
pub const MOST: usize = 10;
/// The inbox a postmortem's answers come to
pub const INBOX: &str = "fingerprint";
/// Two asks on one axis of one piece closer than this are one
const APART: f64 = 4.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ask {
    pub file: PathBuf,
    pub media: String,
    pub a: f64,
    pub b: f64,
    pub axis: String,
    pub words: String,
    /// "disagrees with you" or "least sure"
    pub why: String,
    /// lower is asked first
    pub rank: f32,
}

/// The moments to ask about: where a head's flag goes against a label the person gave on that moment, then where a
/// head is nearest its flag, in units of its flag-to-strong spread; one per axis and piece within APART seconds
pub fn picks(scored: &[Scored], trained: &Trained, fp: &Fingerprint, l: &Ledger, most: usize) -> Vec<Ask> {
    let mut all = Vec::new();
    for s in scored {
        for w in &s.windows {
            for x in w.axes.iter().filter(|x| x.by == "head") {
                let Some(h) = trained.heads.iter().find(|h| h.axis == x.axis) else { continue };
                let Some(t) = h.flag else { continue };
                let said = l.rows.iter().find(|r| r.media == s.media && r.a < w.b && r.b > w.a && !matches!(r.source.as_str(), "cheat" | "damage" | "pair") && (r.axis == x.axis || (r.axis == "*" && !r.bad)));
                let (why, rank) = match said {
                    Some(r) if r.bad != x.flagged => ("disagrees with you", -1.0),
                    Some(_) => continue,
                    None => ("least sure", (x.value - t).abs() / (h.strong.unwrap_or(t + 1.0) - t).abs().max(1e-3)),
                };
                let words = fp.axis(&x.axis).and_then(|a| a.words.first().cloned()).unwrap_or_else(|| x.axis.clone());
                all.push(Ask { file: PathBuf::from(&s.file), media: s.media.clone(), a: w.a, b: w.b, axis: x.axis.clone(), words, why: why.into(), rank });
            }
        }
    }
    all.sort_by(|a, b| a.rank.total_cmp(&b.rank));
    let mut out: Vec<Ask> = Vec::new();
    for x in all {
        if out.len() >= most {
            break;
        }
        if !out.iter().any(|o| o.media == x.media && o.axis == x.axis && (o.a - x.a).abs() < APART) {
            out.push(x);
        }
    }
    out
}

/// The moment cut to a clip to play beside its question (ffmpeg), else None and the whole piece plays
pub fn clip(x: &Ask, out: &Path) -> Option<PathBuf> {
    std::fs::create_dir_all(out.parent()?).ok()?;
    let ok = std::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-ss", &format!("{:.2}", (x.a - 0.5).max(0.0)), "-t", &format!("{:.2}", x.b - x.a + 1.0), "-i"])
        .arg(&x.file)
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
        .arg(out)
        .status()
        .is_ok_and(|s| s.success());
    ok.then(|| out.to_path_buf())
}

/// The asks put to the person through `raea` show, one item; returns the item and keeps the asks under it in
/// DIR/postmortem.json, so the answers can be matched in order
pub fn ask(raea: &Path, dir: &Path, project: &str, asks: &[Ask]) -> Result<String> {
    let mut cmd = std::process::Command::new(raea);
    cmd.args(["show", "--title", &format!("What raea isn't sure of in {project}"), "--inbox", INBOX]);
    for (i, x) in asks.iter().enumerate() {
        let play = clip(x, &dir.join(format!("postmortem/{i}.mp4"))).unwrap_or_else(|| x.file.clone());
        let q = format!("{} at {}: is there {} here?", x.file.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default(), crate::score::clock(x.a), x.words);
        cmd.args(["--question", &q, "--preview"]).arg(&play).args(["--choice", "Yes", "--choice", "No"]);
    }
    let out = cmd.output()?;
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    let item = said.split(|c: char| c.is_whitespace() || "(),".contains(c)).find(|w| w.starts_with("i-show-")).ok_or_else(|| anyhow!("raea show didn't say its item: {said}{}", String::from_utf8_lossy(&out.stderr)))?;
    let mut kept: BTreeMap<String, Vec<Ask>> = std::fs::read(dir.join("postmortem.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    kept.insert(item.to_string(), asks.to_vec());
    std::fs::write(dir.join("postmortem.json"), serde_json::to_vec_pretty(&kept)?)?;
    Ok(item.to_string())
}

/// Every answered ask: the asks kept under each item, matched in order to the answers in the inbox
pub fn answered(dir: &Path, inbox: &str) -> Vec<(Ask, bool)> {
    let kept: BTreeMap<String, Vec<Ask>> = std::fs::read(dir.join("postmortem.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let mut out = Vec::new();
    for line in inbox.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let Some(asks) = v["item"].as_str().and_then(|i| kept.get(i)) else { continue };
        for (k, x) in asks.iter().enumerate() {
            match v["answers"][format!("q{}", k + 1)].as_str() {
                Some("Yes") => out.push((x.clone(), true)),
                Some("No") => out.push((x.clone(), false)),
                _ => {}
            }
        }
    }
    out
}

/// Each head's held-out AUC before and after, for the axes that moved
pub fn moved(before: &Trained, after: &Trained) -> Vec<(String, Option<f64>, Option<f64>)> {
    after
        .heads
        .iter()
        .filter_map(|h| {
            let was = before.heads.iter().find(|b| b.axis == h.axis).and_then(|b| b.held_out);
            (was != h.held_out).then(|| (h.axis.clone(), was, h.held_out))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::{AxisScore, Window};
    use raea_sys_fingerprint::evidence::{Row, width};
    use raea_sys_fingerprint::train::AxisHead;

    fn win(a: f64, value: f32, flagged: bool) -> Window {
        Window { a, b: a + 2.0, axes: vec![AxisScore { axis: "blur".into(), value, by: "head", state: "advice".into(), band: Some("weak"), flagged, gating: false }], seen: None, heard: None, level: -20.0, overall: None }
    }

    #[test]
    fn a_disagreement_comes_first_then_the_least_sure() {
        let fp = raea_sys_fingerprint::taxonomy::base("me");
        let trained = Trained { heads: vec![AxisHead { axis: "blur".into(), flag: Some(0.0), strong: Some(2.0), ..Default::default() }], weighting: None };
        let s = Scored { file: "v1.mp4".into(), media: "h1".into(), dur: 30.0, fingerprint: "me".into(), moments: vec![], windows: vec![win(0.0, 1.5, true), win(10.0, 0.1, true), win(20.0, -1.9, false), win(11.0, 0.15, true)] };
        let mut l = Ledger::default();
        // the person approved the first moment: a flag there disagrees with them
        let r = Row { media: "h1".into(), a: 0.0, b: 2.0, axis: "*".into(), bad: false, weight: 1.0, who: "me".into(), source: "approval".into(), project: "p".into(), kind: None, region: None, words: None, at: 0, pin: None, band: None };
        l.push(r, &vec![0.0; width()]);
        let p = picks(&[s], &trained, &fp, &l, 10);
        assert_eq!(p.iter().map(|x| (x.a, x.why.as_str())).collect::<Vec<_>>(), [(0.0, "disagrees with you"), (10.0, "least sure"), (20.0, "least sure")], "11 s is the same moment as 10 s");
        assert_eq!(picks(&[], &trained, &fp, &l, 10), []);
    }

    #[test]
    fn answers_match_their_asks_in_order() {
        let dir = std::env::temp_dir().join(format!("raea-pm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let x = |a: f64| Ask { file: "v.mp4".into(), media: "h".into(), a, b: a + 2.0, axis: "blur".into(), words: "soft".into(), why: "least sure".into(), rank: 0.0 };
        let kept = BTreeMap::from([("i-show-1".to_string(), vec![x(0.0), x(5.0), x(9.0)])]);
        std::fs::write(dir.join("postmortem.json"), serde_json::to_vec(&kept).unwrap()).unwrap();
        let inbox = "{\"item\": \"i-show-1\", \"answers\": {\"q1\": \"Yes\", \"q3\": \"No\"}}\n{\"item\": \"i-show-other\", \"answers\": {\"q1\": \"Yes\"}}";
        let a = answered(&dir, inbox);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(a, [(x(0.0), true), (x(9.0), false)]);
    }
}
