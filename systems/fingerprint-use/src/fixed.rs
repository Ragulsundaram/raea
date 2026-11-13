//! A fix checked against the flag it claims to fix (docs/fingerprint.feature FP9a). An agent that changes a cut
//! because of a gating flag scores the new cut with `--was OLD`: each of the old cut's gating moments is still there
//! (the same axis flags near it), looks disguised, or is fixed. "Looks disguised" reruns the axis's cheats on the fix:
//! the fixed moment is nearer the axis's disguised defects (a blur sharpened afterwards, a freeze under a slow zoom)
//! than its clean work, so the head may have been fooled rather than the defect fixed. The times are the old cut's,
//! matched within a second either way, so a fix that moves the moment far is read as fixed only by the head.

use crate::score::{Moment, Scored};
use raea_sys_columns::Columns;
use raea_sys_fingerprint::evidence::{Ledger, features, slice};
use raea_sys_fingerprint::taxonomy::Fingerprint;
use serde::Serialize;
use std::collections::BTreeMap;

/// How far either way a flag in the new cut still counts as the old one, seconds
const NEAR: f64 = 1.0;

#[derive(Clone, Debug, Serialize)]
pub struct Fix {
    pub axis: String,
    pub a: f64,
    pub b: f64,
    pub words: String,
    /// "still there", "looks disguised" or "fixed"
    pub verdict: &'static str,
    pub why: String,
}

fn mean(rows: &[&[f32]]) -> Option<Vec<f32>> {
    let n = rows.len();
    (n > 0).then(|| (0..rows[0].len()).map(|j| rows.iter().map(|r| r[j]).sum::<f32>() / n as f32).collect())
}

fn cos(a: &[f32], b: &[f32]) -> f32 {
    let (d, na, nb) = a.iter().zip(b).fold((0.0, 0.0, 0.0), |(d, na, nb), (x, y)| (d + x * y, na + x * x, nb + y * y));
    d / (na.sqrt() * nb.sqrt()).max(1e-9)
}

/// The disguise a moment is nearest, when it is nearer one of the axis's disguised defects than its clean work
pub fn disguised(l: &Ledger, axis: &str, m: raea_sys_columns::window::Modality, row: &[f32]) -> Option<String> {
    let mut kinds: BTreeMap<String, Vec<&[f32]>> = BTreeMap::new();
    let mut clean = Vec::new();
    for (i, r) in l.rows.iter().enumerate() {
        if r.source == "cheat" && r.axis == axis {
            kinds.entry(r.kind.clone().unwrap_or_else(|| "disguised".into())).or_default().push(l.modality(i, m));
        } else if r.source != "cheat" && !r.bad && (r.axis == axis || r.axis == "*") {
            clean.push(l.modality(i, m));
        }
    }
    let clean = cos(row, &mean(&clean)?);
    kinds.into_iter().filter_map(|(k, rows)| Some((k, cos(row, &mean(&rows)?)))).filter(|(_, s)| *s > clean).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(k, _)| k)
}

/// Each of `was`'s gating moments, judged on `now` (whose columns are `c`)
pub fn fixes(was: &Scored, now: &Scored, fp: &Fingerprint, l: &Ledger, c: &Columns) -> Vec<Fix> {
    let still = |m: &Moment| now.moments.iter().any(|n| n.axis == m.axis && n.a <= m.b + NEAR && n.b >= m.a - NEAR);
    was.moments
        .iter()
        .filter(|m| m.gating)
        .map(|m| {
            let base = |verdict, why: String| Fix { axis: m.axis.clone(), a: m.a, b: m.b, words: m.words.clone(), verdict, why };
            if still(m) {
                return base("still there", format!("the new cut still flags {} here", m.axis));
            }
            let (a, b) = (m.a.min(c.dur), m.b.min(c.dur));
            let looks = fp.axis(&m.axis).filter(|_| b > a).and_then(|x| disguised(l, &m.axis, x.modality, &features(c, a, b)[slice(x.modality)]));
            match looks {
                Some(k) => base("looks disguised", format!("nearer its disguised version ({k}) than clean work: fix the cause, not how it reads")),
                None => base("fixed", format!("no {} flag near it now", m.axis)),
            }
        })
        .collect()
}

/// The fixes as lines for the agent, in the person's words
pub fn said(f: &[Fix]) -> String {
    let fixed = f.iter().filter(|x| x.verdict == "fixed").count();
    let mut t = format!("against the cut before: {fixed} of {} flags fixed\n", f.len());
    for x in f {
        t += &format!("  {} {} ({}): {}, {}\n", crate::score::clock(x.a), x.axis, x.words, x.verdict, x.why);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use raea_sys_columns::window::Modality;
    use raea_sys_fingerprint::evidence::{Row, width};

    fn row(axis: &str, source: &str, bad: bool, kind: Option<&str>) -> Row {
        Row { media: "m".into(), a: 0.0, b: 2.0, axis: axis.into(), bad, weight: 1.0, who: "me".into(), source: source.into(), project: "p".into(), kind: kind.map(Into::into), region: None, words: None, at: 0, pin: None, band: None }
    }

    #[test]
    fn a_fix_nearer_a_disguise_than_clean_work_looks_disguised() {
        let m = Modality::ALL[0];
        let s = slice(m);
        let at = |v: &[f32]| {
            let mut f = vec![0.0; width()];
            f[s.start..s.start + v.len()].copy_from_slice(v);
            f
        };
        let mut l = Ledger::default();
        l.push(row("blur", "damage", false, None), &at(&[1.0, 0.0, 0.0]));
        l.push(row("*", "approval", false, None), &at(&[0.9, 0.1, 0.0]));
        l.push(row("blur", "cheat", true, Some("sharpened")), &at(&[0.0, 1.0, 0.0]));
        let fooled = &at(&[0.1, 1.0, 0.0])[s.clone()];
        let clean = &at(&[1.0, 0.05, 0.0])[s.clone()];
        assert_eq!(disguised(&l, "blur", m, fooled).as_deref(), Some("sharpened"));
        assert_eq!(disguised(&l, "blur", m, clean), None);
        assert_eq!(disguised(&l, "glow", m, fooled), None, "an axis with no disguises can't look disguised");
    }
}
