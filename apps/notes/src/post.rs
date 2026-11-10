//! `raea notes post`: each note sent from the raea app gets where it stands shown on its comment (DR5, DR6), through
//! `raea show --on ITEM --working COMMENT`: its asks once restated (`--ask TEXT...`), for the person to correct one in
//! a word, which `raea notes take` writes into the note to be restated again; the note's state for the comment's
//! badge and each ask's (`--status S --states S1,S2`: to restate, to check, failing, to sign, passing, signed); and
//! each clip the person is to sign, under its ask (`--sign N --clip FILE --render HASH`). "done" is never the agent's:
//! a note closes when the person retires it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::state::{self, Standing};
use crate::{Eval, R, place, read};

/// What was last shown for each note, by where it came from (`.raea/notes-shown.json`): its asks, its states and its
/// clips. A note shown so already isn't shown again, so a comment isn't marked working again for nothing.
fn shown_at(project: &Path) -> PathBuf {
    project.join(".raea/notes-shown.json")
}

/// The note from `from` as not shown: the person corrected it, so its next restatement is shown even in the same words
/// (the app takes new asks as the answer to their correction).
pub(crate) fn forget(project: &Path, from: &str) -> R<()> {
    let f = shown_at(project);
    let Ok(t) = std::fs::read_to_string(&f) else { return Ok(()) };
    let mut shown: BTreeMap<String, Vec<String>> = serde_json::from_str(&t).unwrap_or_default();
    if shown.remove(from).is_some() {
        let t = serde_json::to_string_pretty(&shown).map_err(|e| e.to_string())?;
        std::fs::write(&f, t + "\n").map_err(|e| format!("{}: {e}", f.display()))?;
    }
    Ok(())
}

/// Runs raea with these arguments: what it printed, or why it failed.
pub type Run = fn(&[String]) -> R<String>;

/// This raea, run again with `args`.
pub fn run_raea(args: &[String]) -> R<String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe).args(args).output().map_err(|e| e.to_string())?;
    let said = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    if out.status.success() { Ok(said(&out.stdout)) } else { Err(said(&out.stderr).lines().last().unwrap_or("raea show failed").to_string()) }
}

/// The item and comment a note sent from the app came from ("app ITEM cm-N"); a note on an item as a whole has none.
pub(crate) fn comment_of(from: &str) -> Option<(&str, &str)> {
    let mut it = from.strip_prefix("app ")?.split(' ');
    let (item, comment) = (it.next()?, it.next()?);
    (comment.starts_with("cm-") && it.next().is_none()).then_some((item, comment))
}

/// The arguments that show a note on its comment: its asks (none while it's to restate), its state and each ask's.
pub(crate) fn args(item: &str, comment: &str, asks: &[&str], status: &str, states: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = ["show", "--on", item, "--working", comment].map(String::from).to_vec();
    a.extend(asks.iter().flat_map(|t| ["--ask".to_string(), t.to_string()]));
    a.extend(["--status".to_string(), status.to_string()]);
    if !states.is_empty() {
        a.extend(["--states".to_string(), states.join(",")]);
    }
    a
}

/// `raea notes post`: what was shown where, and what wasn't and why; 1 when one couldn't be posted.
pub(crate) fn post(project: &Path, eval: Eval, run: Run) -> (String, i32) {
    let mut out = Vec::new();
    let mut code = 0;
    let mut shown: BTreeMap<String, Vec<String>> = std::fs::read_to_string(shown_at(project)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let before = shown.clone();
    let kept = state::read(project);
    for k in read(project, eval) {
        let file = k.file.display().to_string();
        let n = match k.note {
            Ok(n) => n,
            Err(e) => {
                out.push(format!("{file}: can't be read: {e}"));
                code = 1;
                continue;
            }
        };
        let Some((item, comment)) = comment_of(&n.from) else {
            if n.from.starts_with("app ") {
                out.push(format!("{file}: a note on its item as a whole has no comment to show its asks on: say them to the person"));
            }
            continue;
        };
        let number = place(project, &project.join(&k.file)).map_or(0, |p| p.1);
        let owed = n.flags();
        let (standing, each) = state::of_note(&n, number, &kept);
        // a restatement that still owes something isn't shown: the comment reads "to restate" until it's whole
        let whole = n.state() != "to restate" && owed.is_empty();
        let asks: Vec<&str> = if whole { n.asks.iter().map(|a| a.text.as_str()).collect() } else { Vec::new() };
        let status = if whole { standing.word() } else { "to restate" };
        let states: Vec<&str> = if whole { each.iter().map(Standing::word).collect() } else { Vec::new() };
        // each clip the person is to sign, under its ask: (its number, file, render)
        let clips: Vec<(usize, String, String)> = if whole {
            each.iter().enumerate().filter(|(_, s)| matches!(s, Standing::ToSign(_))).filter_map(|(i, _)| {
                let c = kept.get(&state::id(number, i + 1))?.clip.as_ref().filter(|c| !c.stale)?;
                Some((i + 1, c.file.display().to_string(), c.render.clone()))
            }).collect()
        } else {
            Vec::new()
        };
        let mut said: Vec<String> = asks.iter().map(|a| a.to_string()).collect();
        said.push(format!("status {status} {}", states.join(",")));
        said.extend(clips.iter().map(|(i, _, h)| format!("clip {i} {h}")));
        if shown.get(&n.from) == Some(&said) {
            out.push(format!("{file}: shown on {comment} of {item} as it stands already ({status})"));
            continue;
        }
        let mut calls = vec![args(item, comment, &asks, status, &states)];
        calls.extend(clips.iter().map(|(i, f, h)| ["show", "--on", item, "--working", comment, "--sign", &i.to_string(), "--clip", f, "--render", h].map(String::from).to_vec()));
        match calls.iter().try_for_each(|a| run(a).map(|_| ())) {
            Ok(()) => {
                let what = match (asks.len(), whole) {
                    (_, false) if owed.is_empty() => "to restate".to_string(),
                    (_, false) => format!("to restate: its restatement still owes: {}", owed.join("; ")),
                    (1, true) => format!("its ask, {status}"),
                    (n, true) => format!("its {n} asks, {status}"),
                };
                let signs = if clips.is_empty() { String::new() } else { format!(", with {} clip{} to sign", clips.len(), if clips.len() == 1 { "" } else { "s" }) };
                out.push(format!("{file}: shown on {comment} of {item}: {what}{signs}"));
                shown.insert(n.from.clone(), said);
            }
            Err(e) => {
                out.push(format!("{file}: not posted: {e}"));
                code = 1;
            }
        }
    }
    if shown != before {
        let written = serde_json::to_string_pretty(&shown).map_err(|e| e.to_string()).and_then(|t| std::fs::write(shown_at(project), t + "\n").map_err(|e| e.to_string()));
        if let Err(e) = written {
            out.push(format!("what was shown isn't kept ({}: {e}): the next post shows these again", shown_at(project).display()));
            code = 1;
        }
    }
    if out.is_empty() {
        out.push("no notes from the raea app to post: `raea notes ls` lists them".into());
    }
    (out.join("\n"), code)
}
