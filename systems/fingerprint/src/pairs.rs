//! Pairs (FP6b, FP6c): "this one is better than that one on this axis", from a pick between real options, a hand edit's
//! before and after, or a noted moment that changed in the next version. A pair row's features are the better window's
//! minus the worse one's (source "pair", `bad` true: the first had more of the defect); an axis with pairs adds a
//! Bradley-Terry fit, the logistic fit on the differences with no intercept, held out by project as share ordered right.
//! A pair on "*" (a version pair, which names no axis) counts for every axis at its own weight.

use crate::evidence::Ledger;
use crate::head::{Logistic, Scaler};
use crate::train::{C, folds};
use raea_sys_columns::window::Modality;

/// The pair rows for an axis
pub fn rows(l: &Ledger, axis: &str) -> Vec<usize> {
    (0..l.rows.len()).filter(|&i| l.rows[i].source == "pair" && l.rows[i].pin.is_none() && (l.rows[i].axis == axis || l.rows[i].axis == "*")).collect()
}

/// Bradley-Terry: each difference and its negation, no intercept
pub fn fit(l: &Ledger, rows: &[usize], m: Modality) -> Logistic {
    let d: Vec<Vec<f32>> = rows.iter().map(|&i| l.modality(i, m).to_vec()).collect();
    let neg: Vec<Vec<f32>> = d.iter().map(|v| v.iter().map(|x| -x).collect()).collect();
    let x: Vec<&[f32]> = d.iter().chain(&neg).map(Vec::as_slice).collect();
    let mut sc = Scaler::fit(&x);
    sc.mean.iter_mut().for_each(|v| *v = 0.0);
    let y: Vec<bool> = (0..x.len()).map(|i| i < d.len()).collect();
    let w = l.labels(rows).1.repeat(2);
    Logistic::fit(sc, &x, &y, Some(&w), C, false)
}

/// Share of held-out pairs (by project) the fit orders right
pub fn held_out(l: &Ledger, rows: &[usize], m: Modality) -> Option<f64> {
    let projects = l.projects(&rows);
    if projects.len() < 2 {
        return None;
    }
    let (mut right, mut n) = (0usize, 0usize);
    for fold in folds(&projects) {
        let (test, train): (Vec<usize>, Vec<usize>) = rows.iter().copied().partition(|&i| fold.contains(&l.rows[i].project));
        if train.len() < 2 {
            continue;
        }
        let f = fit(l, &train, m);
        for &i in &test {
            right += usize::from(f.score(l.modality(i, m)) > 0.0);
            n += 1;
        }
    }
    (n > 0).then(|| right as f64 / n as f64)
}

/// A pair's features: the worse window's minus the better one's, so a positive score means the first is worse
pub fn difference(worse: &[f32], better: &[f32]) -> Vec<f32> {
    worse.iter().zip(better).map(|(a, b)| a - b).collect()
}
