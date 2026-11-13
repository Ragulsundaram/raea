//! `raea fingerprint move [--fp CHILD] [--to PARENT] [--axis A] [--source S] [--project P]` (docs/fingerprint.feature
//! FP14d). A note or mark teaches the fingerprint that judged the piece, and a parent never reads a child's evidence
//! (inherit.rs), so one client's taste doesn't move the person's own. Moving labels up is the person's call: the
//! child's own rows that match leave its ledger for a fingerprint it extends (its nearest unless --to names one). A
//! row on an axis the parent hasn't, or on one the child says otherwise, stays: in the parent it would mean something
//! else.

use crate::{Eval, flag};
use anyhow::{Result, anyhow, bail};
use raea_sys_fingerprint::evidence::{Ledger, Row};
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint::inherit;
use raea_sys_fingerprint::taxonomy::Fingerprint;
use std::collections::BTreeSet;

/// The rows `pick` takes from `child` into `parent`; returns how many moved
pub fn rows(child: &mut Ledger, parent: &mut Ledger, pick: &dyn Fn(&Row) -> bool) -> usize {
    let mut n = 0;
    for i in 0..child.rows.len() {
        if pick(&child.rows[i]) {
            parent.push(child.rows[i].clone(), child.features(i));
            n += 1;
        }
    }
    child.retain(|r| !pick(r));
    n
}

/// The axes a row may carry into `parent`: the parent's own and inherited ones, less those the child overrides
pub fn carried(parent: &Fingerprint, eval: Eval, over: &BTreeSet<String>) -> BTreeSet<String> {
    let full = inherit::axes(parent, &inherit::parents(parent, Some(eval)));
    full.axes.iter().map(|a| a.name.clone()).filter(|a| !over.contains(a)).chain(["*".to_string()]).collect()
}

/// Moves the child's matching rows up and writes both ledgers; returns the parent's name and what it did
pub(crate) fn up(dir: &Folder, child: &Fingerprint, args: &[String], eval: Eval) -> Result<(String, String)> {
    let (axis, source, project) = (flag(args, "--axis"), flag(args, "--source"), flag(args, "--project"));
    if axis.is_none() && source.is_none() && project.is_none() {
        bail!("say which labels: --axis A, --source S (note, teach, approval, pair, damage) or --project P");
    }
    let parents = inherit::parents(child, Some(eval));
    let (pdir, parent) = match flag(args, "--to") {
        Some(n) => parents.iter().find(|(_, p)| p.name == n).ok_or_else(|| anyhow!("{} doesn't extend {n}: labels only move up", child.name))?,
        None => parents.first().ok_or_else(|| anyhow!("{} extends nothing, so its labels have nowhere to move up to", child.name))?,
    };
    let keep = carried(parent, eval, &inherit::overrides(child, &parents));
    let wanted = |r: &Row| axis.is_none_or(|a| r.axis == a) && source.is_none_or(|s| r.source == s) && project.is_none_or(|p| r.project == p);
    let (mut lc, mut lp) = (dir.ledger()?, pdir.ledger()?);
    let stay = lc.rows.iter().filter(|r| wanted(r) && !keep.contains(&r.axis)).count();
    let n = rows(&mut lc, &mut lp, &|r| wanted(r) && keep.contains(&r.axis));
    if n == 0 && stay == 0 {
        bail!("none of {}'s own labels match", child.name);
    }
    pdir.write_ledger(&lp)?;
    dir.write_ledger(&lc)?;
    let mut out = format!("{n} labels moved from {} up to {}\n", child.name, parent.name);
    if stay > 0 {
        out += &format!("{stay} stay: their axes are {}'s own, and would mean something else in {}\n", child.name, parent.name);
    }
    Ok((parent.name.clone(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(axis: &str, source: &str) -> Row {
        Row { media: "m".into(), a: 0.0, b: 2.0, axis: axis.into(), bad: true, weight: 1.0, who: "me".into(), source: source.into(), project: "p".into(), kind: None, region: None, words: None, at: 0, pin: None, band: None }
    }

    #[test]
    fn only_picked_rows_move_and_keep_their_features() {
        let w = raea_sys_fingerprint::evidence::width();
        let (mut c, mut p) = (Ledger::default(), Ledger::default());
        c.push(row("blur", "note"), &vec![1.0; w]);
        c.push(row("glow", "note"), &vec![2.0; w]);
        c.push(row("blur", "damage"), &vec![3.0; w]);
        p.push(row("hiss", "teach"), &vec![4.0; w]);
        let n = rows(&mut c, &mut p, &|r| r.source == "note" && r.axis == "blur");
        assert_eq!(n, 1);
        assert_eq!(c.rows.iter().map(|r| r.axis.as_str()).collect::<Vec<_>>(), ["glow", "blur"]);
        assert_eq!((c.features(0)[0], c.features(1)[0]), (2.0, 3.0), "the child's rows keep their own features");
        assert_eq!(p.rows.len(), 2);
        assert_eq!(p.features(1)[0], 1.0, "a moved row carries its features");
    }
}
