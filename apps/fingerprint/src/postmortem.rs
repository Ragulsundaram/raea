//! `raea fingerprint postmortem [PROJECT] [--ask N] [--tally]` (docs/fingerprint.feature FP7a; the picking, the clips
//! and the questions are raea-sys-fingerprint-use's postmortem.rs). It scores every version of the project's pieces with
//! the fingerprint that judges the project and asks at most ten yes/no questions in the app; `--tally` makes each answer
//! an evidence row, retrains, and says how each head's held-out score moved.

use crate::{Eval, flag, has};
use anyhow::{Result, anyhow, bail};
use raea_sys_columns::Encoders;
use raea_sys_fingerprint::evidence::{Row, features};
use raea_sys_fingerprint::retrain;
use raea_sys_fingerprint_use::postmortem::{self as pm, MOST};
use raea_sys_fingerprint_use::score::{Scored, score};
use std::path::{Path, PathBuf};

pub(crate) fn run(enc: &mut Encoders, args: &[String], cwd: &Path, eval: Eval) -> Result<(String, i32)> {
    let project = crate::pick::piece_in(args, cwd).parent().map(Path::to_path_buf).unwrap_or_else(|| cwd.to_path_buf());
    let name = project.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (dir, fp, w) = crate::verbs::open(enc, args, eval, Some(&crate::pick::piece_in(args, cwd)))?;
    let before = dir.trained()?.map(|t| t.0).ok_or_else(|| anyhow!("{} has no trained heads yet: `raea fingerprint bootstrap` or teach it first", fp.name))?;
    if has(args, "--tally") {
        let inbox = std::fs::read_to_string(PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(format!(".raea/inbox/{}.jsonl", pm::INBOX))).unwrap_or_default();
        let answers = pm::answered(&dir.dir, &inbox);
        let mut l = dir.ledger()?;
        // every tally adds every answer again, so tallying twice is safe
        l.retain(|r| r.source != "postmortem");
        for (x, yes) in &answers {
            let c = enc.columns(&x.file)?;
            let (a, b) = (x.a.min(c.dur), x.b.min(c.dur));
            if b - a < 0.5 {
                continue;
            }
            let who = fp.who.clone().unwrap_or_else(|| "me".into());
            let row = Row { media: c.media.clone(), a, b, axis: x.axis.clone(), bad: *yes, weight: 1.0, who, source: "postmortem".into(), project: name.clone(), kind: None, region: None, words: Some(x.words.clone()), at: 0, pin: None, band: None };
            l.push(row, &features(&c, a, b));
        }
        dir.write_ledger(&l)?;
        let after = retrain(&dir, &fp, &w, &l, None)?;
        let f = |v: Option<f64>| v.map_or("-".into(), |v| format!("{v:.2}"));
        let mut out = format!("{} answers are evidence now\n", answers.len());
        for (axis, was, now) in pm::moved(&before, &after) {
            out += &format!("  {axis}: held out {} -> {}\n", f(was), f(now));
        }
        return Ok((out, 0));
    }
    let n = flag(args, "--ask").map_or(Ok(MOST), str::parse).map_err(|_| anyhow!("--ask takes a count"))?.min(MOST);
    let files = crate::notes::videos(&project);
    if files.is_empty() {
        bail!("{} has no videos in its feed to look back on", project.display());
    }
    let scored = files.iter().map(|f| -> Result<Scored> { Ok(score(&fp, Some(&w), Some(&before), &enc.columns(f)?, &f.display().to_string())) }).collect::<Result<Vec<_>>>()?;
    let asks = pm::picks(&scored, &before, &fp, &dir.ledger()?, n);
    if asks.is_empty() {
        return Ok((format!("nothing to ask: on {} versions, no head disagrees with you or sits near its flag\n", files.len()), 0));
    }
    let item = pm::ask(&std::env::current_exe()?, &dir.dir, &name, &asks)?;
    let against = asks.iter().filter(|x| x.why == "disagrees with you").count();
    Ok((format!("{} questions on {} versions put to you in the app ({item}): {against} where it disagrees with you, the rest where it's least sure; `raea fingerprint postmortem --tally` adds your answers\n", asks.len(), files.len()), 0))
}
