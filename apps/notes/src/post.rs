//! `raea notes post`: each restated note sent from the raea app gets its asks shown back on its comment (DR5), through
//! `raea show --on ITEM --working COMMENT --ask TEXT...`, before the work starts. The person reads them there and can
//! correct one in a word; `raea notes take` writes their correction into the note, which is then restated again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{Eval, R, read};

/// What was last shown for each note, by where it came from (`.raea/notes-shown.json`): a note shown with these asks
/// already isn't shown again, so a comment the agent has since settled isn't marked working again.
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

/// The arguments that show a note's asks on its comment.
pub(crate) fn args(item: &str, comment: &str, asks: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = ["show", "--on", item, "--working", comment].map(String::from).to_vec();
    a.extend(asks.iter().flat_map(|t| ["--ask".to_string(), t.to_string()]));
    a
}

/// `raea notes post`: what was shown where, and what wasn't and why; 1 when one couldn't be posted.
pub(crate) fn post(project: &Path, eval: Eval, run: Run) -> (String, i32) {
    let mut out = Vec::new();
    let mut code = 0;
    let mut shown: BTreeMap<String, Vec<String>> = std::fs::read_to_string(shown_at(project)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let before = shown.clone();
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
        let owed = n.flags();
        let Some((item, comment)) = comment_of(&n.from) else {
            if n.from.starts_with("app ") {
                out.push(format!("{file}: a note on its item as a whole has no comment to show its asks on: say them to the person"));
            }
            continue;
        };
        if n.state() == "to restate" || !owed.is_empty() {
            out.push(format!("{file}: not posted: {}", if owed.is_empty() { "it's still to restate".to_string() } else { format!("its restatement still owes: {}", owed.join("; ")) }));
            continue;
        }
        let asks: Vec<&str> = n.asks.iter().map(|a| a.text.as_str()).collect();
        if shown.get(&n.from).is_some_and(|s| s.iter().eq(asks.iter().copied())) {
            out.push(format!("{file}: its asks are shown on {comment} of {item} already"));
            continue;
        }
        match run(&args(item, comment, &asks)) {
            Ok(_) => {
                out.push(format!("{file}: its {} ask{} shown on {comment} of {item}, where the person can correct one", asks.len(), if asks.len() == 1 { "" } else { "s" }));
                shown.insert(n.from.clone(), asks.iter().map(|a| a.to_string()).collect());
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
