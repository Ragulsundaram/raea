//! Pairs from what the person already does (docs/fingerprint.feature FP6b, FP6c). `raea fingerprint pair WORSE BETTER
//! --axis A [--at T] [--then T2]` records a pick between real options or a hand edit's before and after: the better
//! window against the worse on that axis. `raea fingerprint versions V1 V2...` (the files in version order) makes a
//! weak pair (weight 0.5, every axis) of each noted moment whose picture or sound changed in the next version (cosine
//! distance 0.01 or more) and wasn't noted again within 1 s there.

use crate::approve::noted;
use crate::flag;
use anyhow::{Context, Result, anyhow, bail};
use raea_sys_columns::Encoders;
use raea_sys_columns::window::Modality;
use raea_sys_fingerprint::evidence::{Ledger, Row, features, slice};
use raea_sys_fingerprint::pairs::difference;
use std::path::Path;

fn row(media: &str, a: f64, b: f64, axis: &str, weight: f32, who: &str, project: String, words: Option<String>) -> Row {
    Row { media: media.into(), a, b, axis: axis.into(), bad: true, weight, who: who.into(), source: "pair".into(), project, kind: None, region: None, words, at: 0, pin: None }
}


/// `pair`: one row, the worse window's features minus the better one's
pub(crate) fn pair(enc: &mut Encoders, l: &mut Ledger, who: &str, args: &[String], cwd: &Path, project: impl Fn(&Path) -> String) -> Result<String> {
    let f = crate::named(args, cwd)?;
    let [worse, better] = f.as_slice() else { bail!("raea fingerprint pair WORSE BETTER --axis A [--at T] [--then T2]") };
    let axis = flag(args, "--axis").ok_or_else(|| anyhow!("--axis A: the axis the better one is better on"))?;
    let span: f64 = flag(args, "--span").map_or(Ok(2.0), str::parse).context("--span takes seconds")?;
    let t: f64 = flag(args, "--at").map_or(Ok(span / 2.0), str::parse).context("--at takes seconds")?;
    let t2: f64 = flag(args, "--then").map_or(Ok(t), str::parse).context("--then takes seconds")?;
    let (cw, cb) = (enc.columns(worse)?, enc.columns(better)?);
    let win = |dur: f64, t: f64| {
        let a = (t - span / 2.0).clamp(0.0, (dur - span).max(0.0));
        (a, (a + span).min(dur))
    };
    let ((a, b), (a2, b2)) = (win(cw.dur, t), win(cb.dur, t2));
    let d = difference(&features(&cw, a, b), &features(&cb, a2, b2));
    l.push(row(&cw.media, a, b, axis, 1.0, who, flag(args, "--project").map_or_else(|| project(worse), str::to_string), flag(args, "--words").map(str::to_string)), &d);
    Ok(format!("paired on {axis}: {} at {a:.1} s is worse than {} at {a2:.1} s\n", worse.display(), better.display()))
}

fn distance(x: &[f32], y: &[f32]) -> f32 {
    let d: f32 = x.iter().zip(y).map(|(a, b)| a * b).sum();
    1.0 - d / (x.iter().map(|a| a * a).sum::<f32>().sqrt() * y.iter().map(|b| b * b).sum::<f32>().sqrt()).max(1e-9)
}

/// Whether a noted moment changed in the next version and wasn't noted there again
pub(crate) fn weak(before: &[f32], after: &[f32], t: f64, next_notes: &[f64]) -> bool {
    let changed = [Modality::Picture, Modality::Sound].iter().any(|&m| distance(&before[slice(m)], &after[slice(m)]) >= 0.01);
    changed && !next_notes.iter().any(|n| (n - t).abs() <= 1.0)
}

/// `versions`: weak pairs along a chain of versions
pub(crate) fn versions(enc: &mut Encoders, l: &mut Ledger, who: &str, args: &[String], cwd: &Path, project: impl Fn(&Path) -> String) -> Result<String> {
    let f = crate::named(args, cwd)?;
    if f.len() < 2 {
        bail!("raea fingerprint versions V1 V2... takes the versions in order");
    }
    let mut made = 0;
    for w in f.windows(2) {
        let (now, next) = (&w[0], &w[1]);
        let (cn, cx) = (enc.columns(now)?, enc.columns(next)?);
        let next_notes = noted(next);
        for t in noted(now) {
            let (a, b) = ((t - 1.5).max(0.0), (t + 0.5).min(cn.dur.min(cx.dur)));
            if b - a < 1.0 {
                continue;
            }
            let (fb, fa) = (features(&cn, a, b), features(&cx, a, b));
            if weak(&fb, &fa, t, &next_notes) {
                l.push(row(&cn.media, a, b, "*", 0.5, who, flag(args, "--project").map_or_else(|| project(now), str::to_string), None), &difference(&fb, &fa));
                made += 1;
            }
        }
    }
    Ok(format!("{made} weak pairs from {} versions: noted moments the next version changed and nobody noted again\n", f.len()))
}

#[cfg(test)]
mod tests {
    use raea_sys_fingerprint::evidence::width;

    #[test]
    fn a_changed_moment_not_noted_again_is_a_weak_pair() {
        let a = vec![1.0; width()];
        let mut b = a.clone();
        assert!(!super::weak(&a, &b, 5.0, &[]), "unchanged: no pair");
        b.iter_mut().enumerate().for_each(|(i, x)| *x = if i % 2 == 0 { 1.0 } else { -1.0 });
        assert!(super::weak(&a, &b, 5.0, &[]));
        assert!(!super::weak(&a, &b, 5.0, &[5.6]), "noted again within 1 s: no pair");
        assert!(super::weak(&a, &b, 5.0, &[7.0]));
    }
}
