//! Training (FP2c, FP2d): each axis's head from its evidence, its held-out by project score, its bands and its
//! cheats, and whether it has earned the right to gate; then the person's weighting over the axes. Runs after every
//! new row: milliseconds a head on the CPU.

use crate::evidence::{Ledger, slice};
use crate::head::{Logistic, Scaler, auc, band, quantile};
use crate::taxonomy::Fingerprint;
use crate::words::Words;
use raea_sys_columns::window::Modality;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const C: f64 = 0.5;
/// The gate (FP2d): labels of each polarity, projects, held-out AUC, and each cheat's catch rate
pub const GATE_LABELS: usize = 16;
pub const GATE_PROJECTS: usize = 3;
pub const GATE_AUC: f64 = 0.8;
pub const GATE_CHEAT: f64 = 0.8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AxisHead {
    pub axis: String,
    pub modality: Option<Modality>,
    /// the learned head: kept in heads.safetensors, not in report.json
    #[serde(skip)]
    pub head: Option<Logistic>,
    /// labels by source: [defect, clean]
    pub labels: BTreeMap<String, [usize; 2]>,
    pub projects: usize,
    /// the learned head's AUC on projects it never saw
    pub held_out: Option<f64>,
    /// the word head's AUC on the same labels, with no training
    pub words: Option<f64>,
    /// the word score that flags 10% of clean windows
    pub word_flag: Option<f32>,
    /// each cheat's share caught at the threshold that flags 10% of clean held-out work
    pub cheats: BTreeMap<String, f64>,
    /// "gating", "advice" or "words" (only the word head)
    pub state: String,
    pub why: String,
    /// decision values above which a flag was right 0.9 (strong) and 0.7 (plausible) of the time, held out
    pub strong: Option<f32>,
    pub plausible: Option<f32>,
    /// the decision value that flags 10% of clean held-out work
    pub flag: Option<f32>,
    /// what this head can't judge (earshot's LIMITS.md)
    pub cannot: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Trained {
    pub heads: Vec<AxisHead>,
    /// the person's weighting over the axes' scores, once they've given 8 overall labels of each kind
    #[serde(skip)]
    pub weighting: Option<Logistic>,
}

/// The modality's scaler: unlabelled statistics over every row that isn't a cheat
fn scaler(l: &Ledger, rows: &[usize], m: Modality) -> Scaler {
    let x: Vec<&[f32]> = rows.iter().map(|&i| l.modality(i, m)).collect();
    Scaler::fit(&x)
}

/// The rows' labels and weights
fn labels(l: &Ledger, rows: &[usize]) -> (Vec<bool>, Vec<f32>) {
    let (mut y, mut w) = (Vec::with_capacity(rows.len()), Vec::with_capacity(rows.len()));
    for &i in rows {
        y.push(l.rows[i].bad);
        w.push(l.rows[i].weight);
    }
    (y, w)
}

/// How many of the rows say the defect is there, and how many say it isn't
fn counts(l: &Ledger, rows: &[usize]) -> (usize, usize) {
    let bad = rows.iter().filter(|&&i| l.rows[i].bad).count();
    (bad, rows.len() - bad)
}

fn fit(l: &Ledger, rows: &[usize], m: Modality, sc: Scaler) -> Logistic {
    let x: Vec<&[f32]> = rows.iter().map(|&i| l.modality(i, m)).collect();
    let (y, w) = labels(l, rows);
    Logistic::fit(sc, &x, &y, Some(&w), C, true)
}

/// Folds over projects (FP0b's: sorted, every fifth), or one project each when there are five or fewer
pub fn folds(projects: &BTreeSet<String>) -> Vec<Vec<String>> {
    let p: Vec<String> = projects.iter().cloned().collect();
    let k = p.len().min(5);
    (0..k).map(|i| p.iter().skip(i).step_by(k).cloned().collect()).collect()
}

fn limits(m: Modality) -> Vec<String> {
    let mut v = vec!["why a moment is wrong, beyond naming the axis".to_string(), "whether the piece works as a whole, or for its audience".to_string()];
    v.push(match m {
        Modality::Picture => "where in the frame (needs marks: FP12)".into(),
        Modality::Look => "whether a look was meant (a stylised grade reads as a defect)".into(),
        Modality::Change => "whether stillness or a cut was meant".into(),
        Modality::Sound | Modality::Events => "which sound in a mix it means, or whether it was meant".into(),
    });
    v
}

pub fn axis(fp: &Fingerprint, l: &Ledger, words: &Words, name: &str) -> AxisHead {
    let Some(ax) = fp.axis(name) else { return AxisHead { axis: name.into(), state: "none".into(), why: "not in the taxonomy".into(), ..Default::default() } };
    let m = ax.modality;
    let rows = l.for_axis(name);
    let mut h = AxisHead { axis: name.into(), modality: Some(m), cannot: limits(m), ..Default::default() };
    for &i in &rows {
        h.labels.entry(l.rows[i].source.clone()).or_default()[usize::from(!l.rows[i].bad)] += 1;
    }
    let (pos, neg) = counts(l, &rows);
    let projects: BTreeSet<String> = rows.iter().map(|&i| l.rows[i].project.clone()).collect();
    h.projects = projects.len();
    let plain: Vec<usize> = (0..l.rows.len()).filter(|&i| l.rows[i].source != "cheat").collect();
    let worded = words.axes.iter().any(|a| a.0 == name);
    if worded {
        let s: Vec<(f32, bool)> = rows.iter().filter_map(|&i| words.z(name, l.features(i)).map(|z| (z, l.rows[i].bad))).collect();
        let (p, n): (Vec<f32>, Vec<f32>) = (s.iter().filter(|x| x.1).map(|x| x.0).collect(), s.iter().filter(|x| !x.1).map(|x| x.0).collect());
        h.words = Some(auc(&p, &n)).filter(|a| a.is_finite());
        h.word_flag = Some(quantile(&n, 0.9)).filter(|v| v.is_finite());
    }
    if pos < 2 || neg < 2 {
        h.state = if worded { "words".into() } else { "none".into() };
        h.why = format!("{pos} defect and {neg} clean labels: the head needs some of each");
        return h;
    }
    h.head = Some(fit(l, &rows, m, scaler(l, &plain, m)));
    // held out by project
    let (mut hp, mut hn) = (Vec::new(), Vec::new());
    let mut caught: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    if projects.len() >= 2 {
        for fold in folds(&projects) {
            let held = |i: usize| fold.contains(&l.rows[i].project);
            let (test, train): (Vec<usize>, Vec<usize>) = rows.iter().copied().partition(|&i| held(i));
            if matches!(counts(l, &train), (0, _) | (_, 0)) {
                continue;
            }
            let base: Vec<usize> = plain.iter().copied().filter(|&i| !held(i)).collect();
            let f = fit(l, &train, m, scaler(l, &base, m));
            let (mut fp_, mut fn_) = (Vec::new(), Vec::new());
            for &i in &test {
                let s = f.score(l.modality(i, m));
                if l.rows[i].bad { fp_.push(s) } else { fn_.push(s) }
            }
            let thr = quantile(&fn_, 0.9);
            for (i, r) in l.rows.iter().enumerate().filter(|(_, r)| r.source == "cheat" && r.axis == name && fold.contains(&r.project)) {
                let e = caught.entry(r.kind.clone().unwrap_or_default()).or_default();
                e.0 += usize::from(f.score(l.modality(i, m)) > thr);
                e.1 += 1;
            }
            hp.extend(fp_);
            hn.extend(fn_);
        }
    }
    h.held_out = Some(auc(&hp, &hn)).filter(|a| a.is_finite());
    h.cheats = caught.into_iter().map(|(k, (c, n))| (k, c as f64 / n.max(1) as f64)).collect();
    h.strong = band(&hp, &hn, 0.9);
    h.plausible = band(&hp, &hn, 0.7);
    h.flag = Some(quantile(&hn, 0.9)).filter(|v| v.is_finite());
    let mut why = Vec::new();
    if pos < GATE_LABELS || neg < GATE_LABELS {
        why.push(format!("{pos} defect and {neg} clean labels (needs {GATE_LABELS} of each)"));
    }
    if h.projects < GATE_PROJECTS {
        why.push(format!("labels from {} projects (needs {GATE_PROJECTS})", h.projects));
    }
    match h.held_out {
        Some(a) if a < GATE_AUC => why.push(format!("held-out AUC {a:.2} (needs {GATE_AUC})")),
        None => why.push("no held-out score yet".into()),
        _ => {}
    }
    for (k, c) in &h.cheats {
        if *c < GATE_CHEAT {
            why.push(format!("the {k} cheat is caught {c:.2} (needs {GATE_CHEAT})"));
        }
    }
    h.state = if why.is_empty() { "gating".into() } else { "advice".into() };
    h.why = if why.is_empty() { "earned: enough labels, held out by project, cheats caught".into() } else { why.join("; ") };
    h
}

/// Every axis's head, then the person's weighting
pub fn train(fp: &Fingerprint, l: &Ledger, words: &Words) -> Trained {
    let heads: Vec<AxisHead> = fp.axes.iter().map(|a| axis(fp, l, words, &a.name)).collect();
    // the weighting: overall labels the person gave (axis "*", their own sources), read through every head
    let overall: Vec<usize> = (0..l.rows.len()).filter(|&i| l.rows[i].axis == "*" && matches!(l.rows[i].source.as_str(), "teach" | "note" | "approval" | "postmortem")).collect();
    let (p, n) = counts(l, &overall);
    let weighting = (p >= 8 && n >= 8).then(|| {
        let x: Vec<Vec<f32>> = overall.iter().map(|&i| scores(&heads, l.features(i))).collect();
        let xr: Vec<&[f32]> = x.iter().map(|v| v.as_slice()).collect();
        Logistic::fit(Scaler::fit(&xr), &xr, &labels(l, &overall).0, None, C, true)
    });
    Trained { heads, weighting }
}

/// Each head's decision value on a row's features (0 for an axis without a learned head)
pub fn scores(heads: &[AxisHead], row: &[f32]) -> Vec<f32> {
    heads.iter().map(|h| match (&h.head, h.modality) {
        (Some(f), Some(m)) => f.score(&row[slice(m)]),
        _ => 0.0,
    }).collect()
}
