//! Approvals are positives (docs/fingerprint.feature FP6a): an approved cut, a picture lock, a publish or a golden
//! reference says every moment of it is clean on every axis, except where the person left a note. `raea fingerprint
//! approve FILE... [--project P]` adds a clean row on every axis ("*") for each 2 s window of each file, skipping the
//! windows within 2 s of a pinned note on that file in its project's feed (.raea/feed/*.json), since a note there may
//! name a defect. Notes aren't read into labels here; that is FP5a.

use crate::bootstrap::WIN;
use crate::{flag, words};
use anyhow::{Result, anyhow};
use raea_sys_columns::Encoders;
use raea_sys_fingerprint::evidence::{Row, features};
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint::taxonomy::Fingerprint;
use std::path::{Path, PathBuf};

/// The seconds a note was pinned at on `file`, from every post in its project's feed that shows a file of that name
pub(crate) fn noted(file: &Path) -> Vec<f64> {
    let Some(name) = file.file_name().and_then(|n| n.to_str()) else { return Vec::new() };
    let Some(feed) = file.ancestors().map(|d| d.join(".raea/feed")).find(|d| d.is_dir()) else { return Vec::new() };
    let mut at = Vec::new();
    for e in std::fs::read_dir(feed).into_iter().flatten().flatten() {
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(post) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let shows = post["media"].as_array().into_iter().flatten().any(|m| m["path"].as_str().is_some_and(|p| p.ends_with(name)));
        if shows {
            at.extend(post["notes"].as_array().into_iter().flatten().filter_map(|n| n["pin"]["t"].as_f64()));
        }
    }
    at
}

/// The windows an approval labels clean: every WIN seconds of the piece, away from its notes
pub(crate) fn windows(dur: f64, notes: &[f64]) -> Vec<(f64, f64)> {
    let n = (dur / WIN).floor() as usize;
    (0..n).map(|i| (i as f64 * WIN, (i + 1) as f64 * WIN)).filter(|(a, b)| !notes.iter().any(|t| *t > a - WIN && *t < b + WIN)).collect()
}

pub(crate) fn approve(enc: &mut Encoders, dir: &Folder, fp: &Fingerprint, args: &[String], cwd: &Path, project: impl Fn(&Path) -> String) -> Result<(String, usize)> {
    let files: Vec<PathBuf> = words(args).map_err(|e| anyhow!(e))?.iter().map(|f| if Path::new(f).is_absolute() { PathBuf::from(f) } else { cwd.join(f) }).collect();
    if files.is_empty() {
        return Err(anyhow!("which approved work? raea fingerprint approve FILE..."));
    }
    let mut l = dir.ledger()?;
    let mut out = String::new();
    let mut added = 0;
    for file in &files {
        let c = enc.columns(file)?;
        // an approval given again replaces the last one for that media
        l.retain(|r| !(r.source == "approval" && r.media == c.media));
        let notes = noted(file);
        let w = windows(c.dur, &notes);
        for (a, b) in &w {
            let row = Row {
                media: c.media.clone(),
                a: *a,
                b: *b,
                axis: "*".into(),
                bad: false,
                weight: 1.0,
                who: fp.who.clone().unwrap_or_else(|| "me".into()),
                source: "approval".into(),
                project: flag(args, "--project").map_or_else(|| project(file), str::to_string),
                kind: None,
                region: None,
                words: None,
                at: 0,
                pin: None,
            };
            l.push(row, &features(&c, *a, *b));
        }
        added += w.len();
        out += &format!("approved {}: {} clean windows on every axis{}\n", file.display(), w.len(), if notes.is_empty() { String::new() } else { format!(", {} noted moments left out", notes.len()) });
    }
    dir.write_ledger(&l)?;
    Ok((out, added))
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_approval_leaves_out_the_windows_near_a_note() {
        assert_eq!(super::windows(9.0, &[]), vec![(0.0, 2.0), (2.0, 4.0), (4.0, 6.0), (6.0, 8.0)]);
        // a note at 5 s: the windows within 2 s of it go
        assert_eq!(super::windows(12.0, &[5.0]), vec![(0.0, 2.0), (8.0, 10.0), (10.0, 12.0)]);
    }
}
