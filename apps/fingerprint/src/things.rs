//! `raea fingerprint things FILE [--project P] [--json]` (docs/fingerprint.feature FP13a, FP13c): a piece's shots,
//! their things (a render's layers, and every mark a person drew on a note of it in P's feed, P the folder it runs in
//! by default) and the camera's path, with what floats, passes through, lurches or is empty
//! (raea-sys-fingerprint-use things.rs and geometry.rs). `--json` prints it all for `raea check --things`.

use crate::{flag, has};
use anyhow::Result;
use raea_sys_columns::Encoders;
use raea_sys_fingerprint_use::things::{Mark, things};
use std::path::{Path, PathBuf};

pub(crate) fn run(enc: &mut Encoders, args: &[String], cwd: &Path) -> Result<(String, i32)> {
    let file = cwd.join(args.first().filter(|a| !a.starts_with("--")).ok_or_else(|| anyhow::anyhow!("things FILE"))?);
    let project = flag(args, "--project").map_or_else(|| cwd.to_path_buf(), |p| cwd.join(p));
    let me = file.canonicalize().ok();
    let marks: Vec<Mark> = crate::notes::pinned(&project)
        .into_iter()
        .filter(|r| r.media.canonicalize().ok() == me)
        .filter_map(|r| Some(Mark { name: r.text.chars().take(40).collect(), t: r.t, a: (r.t - 1.0).max(0.0), b: r.to.unwrap_or(r.t + 1.0), region: r.region? }))
        .collect();
    let t = things(enc, &file, &marks)?;
    if has(args, "--json") {
        return Ok((serde_json::to_string_pretty(&t)?, 0));
    }
    let n: usize = t.shots.iter().map(|s| s.things.len()).sum();
    let mut out = format!("{}: {} shots, {n} things seen ({} marked), {} findings\n", PathBuf::from(&t.file).display(), t.shots.len(), marks.len(), t.issues.len());
    for i in &t.issues {
        out += &format!("  {:>6.1} s  {}  {}\n", i.at, i.rule, i.says);
    }
    Ok((out, 0))
}
