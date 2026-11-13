//! A child fingerprint (FP14b): one that `extends` another has its parent's axes plus its own, and trains on its
//! parent's evidence as well as its own, its own counted twice. An axis of its own with a parent's axis's name but
//! other words, polarity or check overrides it, and learns from the child's own labels only. The parent never reads
//! a child's evidence: one client's taste doesn't move the person's own. The 2x is measured, not trusted: for each axis
//! the child has labels on, held out by the child's own projects, training on both against its own labels alone and
//! the parent's alone.

use crate::evidence::Ledger;
use crate::folder::{Eval, Folder};
use crate::head::{Logistic, Scaler, auc};
use crate::taxonomy::{Axis, Fingerprint};
use crate::train::{C, folds};
use serde::Serialize;
use std::collections::BTreeSet;

/// How much a child's own label counts against an inherited one
pub const OWN: f32 = 2.0;
/// How far up `extends` is followed
const DEPTH: usize = 8;

/// The fingerprints this one extends, nearest first; a missing or circular parent ends the chain
pub fn parents(fp: &Fingerprint, eval: Option<Eval>) -> Vec<(Folder, Fingerprint)> {
    let mut out: Vec<(Folder, Fingerprint)> = Vec::new();
    let mut seen = BTreeSet::from([fp.name.clone()]);
    let mut next = fp.extends.clone();
    while let Some(name) = next.take() {
        if out.len() >= DEPTH || !seen.insert(name.clone()) {
            break;
        }
        let dir = Folder::named(&name);
        let Ok(p) = dir.read(eval) else { break };
        next = p.extends.clone();
        out.push((dir, p));
    }
    out
}

/// Whether the child's axis says something other than the parent's of the same name
fn differs(a: &Axis, b: &Axis) -> bool {
    a.words != b.words || a.polarity != b.polarity || a.check != b.check || a.modality != b.modality
}

/// The axes the child overrides: its own, under a name a parent has, saying something else
pub fn overrides(fp: &Fingerprint, parents: &[(Folder, Fingerprint)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for a in &fp.axes {
        for (_, p) in parents {
            if let Some(b) = p.axis(&a.name) {
                if differs(a, b) {
                    out.insert(a.name.clone());
                }
                break;
            }
        }
    }
    out
}

/// The child's axes, then every parent's axis it doesn't name, so an axis a parent gains later reaches it
pub fn axes(fp: &Fingerprint, parents: &[(Folder, Fingerprint)]) -> Fingerprint {
    let mut f = fp.clone();
    for (_, p) in parents {
        for a in &p.axes {
            if f.axis(&a.name).is_none() {
                f.axes.push(a.clone());
            }
        }
    }
    f
}

/// The rows a child trains on: its own, counted OWN times, then each parent's, except on the axes it overrides.
/// Returns the ledger and how many of its first rows are the child's own.
pub fn pooled(own: &Ledger, parents: &[(Folder, Fingerprint)], over: &BTreeSet<String>) -> (Ledger, usize) {
    let mut l = Ledger::default();
    let scale = if parents.is_empty() { 1.0 } else { OWN };
    for i in 0..own.rows.len() {
        let mut r = own.rows[i].clone();
        r.weight *= scale;
        l.push(r, own.features(i));
    }
    let n = l.rows.len();
    for (dir, _) in parents {
        let Ok(p) = dir.ledger() else { continue };
        for i in 0..p.rows.len() {
            if !over.contains(&p.rows[i].axis) {
                l.push(p.rows[i].clone(), p.features(i));
            }
        }
    }
    (l, n)
}

/// One axis, held out by the child's own projects: AUC training on both, on its own labels alone, on the parent's alone
#[derive(Clone, Debug, Default, Serialize)]
pub struct Measured {
    pub axis: String,
    pub own_labels: usize,
    pub both: Option<f64>,
    pub own: Option<f64>,
    pub parent: Option<f64>,
}

/// FP14b's measure, for every axis the child has labels on of both kinds
pub fn measure(fp: &Fingerprint, l: &Ledger, own: usize) -> Vec<Measured> {
    let mut out = Vec::new();
    for a in &fp.axes {
        let rows = l.for_axis(&a.name);
        let mine: Vec<usize> = rows.iter().copied().filter(|&i| i < own).collect();
        let (bad, clean) = l.counts(&mine);
        if bad < 2 || clean < 2 || own == l.rows.len() {
            continue;
        }
        let projects = l.projects(&mine);
        if projects.len() < 2 {
            continue;
        }
        let m = a.modality;
        let score = |pick: &dyn Fn(usize) -> bool| -> Option<f64> {
            let (mut p, mut n) = (Vec::new(), Vec::new());
            for fold in folds(&projects) {
                let held = |i: usize| i < own && fold.contains(&l.rows[i].project);
                let train: Vec<usize> = rows.iter().copied().filter(|&i| !held(i) && pick(i)).collect();
                let (tb, tn) = l.counts(&train);
                if tb == 0 || tn == 0 {
                    continue;
                }
                let x: Vec<&[f32]> = train.iter().map(|&i| l.modality(i, m)).collect();
                let (y, w) = l.labels(&train);
                let f = Logistic::fit(Scaler::fit(&x), &x, &y, Some(&w), C, true);
                for &i in mine.iter().filter(|&&i| held(i)) {
                    let s = f.score(l.modality(i, m));
                    if l.rows[i].bad { p.push(s) } else { n.push(s) }
                }
            }
            Some(auc(&p, &n)).filter(|a| a.is_finite())
        };
        let both = score(&|_| true);
        let own_ = score(&|i| i < own);
        let parent = score(&|i| i >= own);
        out.push(Measured { axis: a.name.clone(), own_labels: mine.len(), both, own: own_, parent });
    }
    out
}
