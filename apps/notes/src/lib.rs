//! A person's notes on a project (docs/direction.feature DR5). `raea notes take` writes each note the person sends
//! from the raea app (an edit comment with its pin, or a note on an item) or says in a session to a file,
//! notes/<round>/<id>.ts, with their words verbatim and no asks. The agent restates each as its asks before anything
//! in the work changes, and `raea notes ls` says where each stands and what its restatement still owes. `raea notes
//! post` shows the asks on the person's comments in the app, and their corrections come back with the next take. In a
//! project's world each note is an entity with a Note, for the checks that hold it (DR6).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use raea_core_facts::{Facts, Note, record};
use serde_json::json;

mod post;
mod take;

pub const USAGE: &str = "raea notes take [PROJECT] [--inbox NAME] [--said WORDS]... | raea notes ls [PROJECT] [--json] | raea notes post [PROJECT]   (a person's notes as files the work is held to: take writes each note sent from the raea app or said in a session to notes/ROUND/ID.ts, the words verbatim, to be restated as asks before anything changes, and writes in each correction the person made to a restatement; ls says where each note stands, to restate or to check, and what its restatement still owes; post shows each restated note's asks on its comment in the raea app, for the person to correct)";

/// Evaluates a .ts file's default export to its JSON (the CLI's spec host).
pub type Eval = fn(&Path) -> Result<String, String>;

pub(crate) type R<T> = Result<T, String>;

/// The flags `raea notes` takes, and whether each takes a value.
const FLAGS: [(&str, bool); 3] = [("--inbox", true), ("--said", true), ("--json", false)];

/// One note file: where it is in the project, and the note, or why it can't be read.
pub struct Kept {
    pub file: PathBuf,
    pub note: R<Note>,
}

/// `raea notes VERB ...`, printed; its exit code (1: a note that can't be read, 2: the command was wrong).
pub fn cli(args: &[String], eval: Eval) -> i32 {
    match run(args, eval) {
        Ok((out, code)) => {
            println!("{out}");
            code
        }
        Err(e) => {
            eprintln!("raea notes: {e}");
            2
        }
    }
}

fn run(args: &[String], eval: Eval) -> R<(String, i32)> {
    let Some(verb) = args.first().map(String::as_str).filter(|v| !matches!(*v, "--help" | "-h" | "help")) else {
        return Ok((USAGE.into(), 0));
    };
    let rest = &args[1..];
    let mut target = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match FLAGS.iter().find(|(f, _)| f == a) {
            Some((_, true)) => {
                it.next().ok_or_else(|| format!("{a} takes a value\n{USAGE}"))?;
            }
            Some(_) => {}
            None if a.starts_with("--") => return Err(format!("no flag {a}\n{USAGE}")),
            None if target.is_none() => target = Some(a.as_str()),
            None => return Err(format!("one project at a time ({a} is a second)\n{USAGE}")),
        }
    }
    let target = target.unwrap_or(".");
    let project = record::up(Path::new(target), ".raea/project").ok_or_else(|| format!("{target} isn't in a raea project (no .raea/project at or above it)\n{USAGE}"))?;
    match verb {
        "take" => take::take(&project, rest).map(|s| (s, 0)),
        "ls" => Ok(ls(&project, rest.iter().any(|a| a == "--json"), eval)),
        "post" => Ok(post::post(&project, eval, post::run_raea)),
        v => Err(format!("no verb {v}\n{USAGE}")),
    }
}

/// The values a flag was given, in order.
pub(crate) fn values<'a>(args: &'a [String], flag: &str) -> Vec<&'a str> {
    args.iter().zip(args.iter().skip(1)).filter_map(|(f, v)| (f == flag).then_some(v.as_str())).collect()
}

/// A note file's round and number: notes/2/n7.ts is (2, 7); anything else under notes/ isn't a note.
pub(crate) fn place(project: &Path, f: &Path) -> Option<(u32, u32)> {
    let rel = f.strip_prefix(project.join("notes")).ok()?;
    let round = rel.parent()?.to_str()?.parse().ok()?;
    let n = rel.file_name()?.to_str()?.strip_prefix('n')?.strip_suffix(".ts")?.parse().ok()?;
    Some((round, n))
}

/// The project's note files, by round and number.
pub fn files(project: &Path) -> Vec<PathBuf> {
    let dirs = std::fs::read_dir(project.join("notes")).into_iter().flatten().flatten().map(|e| e.path());
    let mut out: Vec<((u32, u32), PathBuf)> = dirs.flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path())).filter_map(|f| Some((place(project, &f)?, f))).collect();
    out.sort();
    out.into_iter().map(|(_, f)| f).collect()
}

/// Every note in the project, read through its file's default export.
pub fn read(project: &Path, eval: Eval) -> Vec<Kept> {
    let of = |f: &Path| eval(f).and_then(|j| serde_json::from_str::<Note>(&j).map_err(|e| format!("not a note ({e})")));
    files(project).into_iter().map(|f| Kept { note: of(&f), file: f.strip_prefix(project).map(Path::to_path_buf).unwrap_or(f) }).collect()
}

/// The notes as entities in a world, one with a Note each.
pub fn spawn(world: &mut Facts, notes: &[Kept]) {
    for n in notes.iter().filter_map(|k| k.note.as_ref().ok()) {
        let e = world.spawn(None);
        world.put(e, n);
    }
}

/// A note's first line, quoted and cut to `n` characters.
pub(crate) fn quoted(words: &str, n: usize) -> String {
    let line = words.lines().next().unwrap_or("").trim();
    if line.chars().count() > n { format!("\"{}…\"", line.chars().take(n).collect::<String>().trim_end()) } else { format!("\"{line}\"") }
}

/// `raea notes ls`: each note with where it stands, its asks and what its restatement still owes; 1 when a note
/// can't be read.
fn ls(project: &Path, as_json: bool, eval: Eval) -> (String, i32) {
    let notes = read(project, eval);
    let code = i32::from(notes.iter().any(|k| k.note.is_err()));
    if as_json {
        let v: Vec<_> = notes
            .iter()
            .map(|k| match &k.note {
                Ok(n) => json!({ "file": k.file, "note": n, "state": n.state(), "flags": n.flags() }),
                Err(e) => json!({ "file": k.file, "error": e }),
            })
            .collect();
        return (serde_json::to_string_pretty(&v).unwrap_or_default(), code);
    }
    if notes.is_empty() {
        return ("no notes yet: `raea notes take` writes each one the person sends from the raea app or says".into(), 0);
    }
    let mut out = Vec::new();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for k in &notes {
        let file = k.file.display();
        let Ok(n) = &k.note else {
            out.push(format!("{file}: can't be read: {}", k.note.as_ref().err().map_or("", String::as_str)));
            *tally.entry("can't be read").or_default() += 1;
            continue;
        };
        *tally.entry(n.state()).or_default() += 1;
        out.push(format!("{file}: round {}, {}: {}", n.round, n.state(), quoted(n.words.first().map_or("", String::as_str), 90)));
        for a in &n.asks {
            let medium = a.medium.as_deref().map(|m| format!(" [{m}]")).unwrap_or_default();
            let not = if a.not.is_empty() { String::new() } else { format!("; not: {}", a.not.join(", ")) };
            out.push(format!("  - {}{medium}{not}", a.text));
        }
        out.extend(n.flags().into_iter().map(|f| format!("  ! {f}")));
    }
    let counts: Vec<String> = tally.iter().map(|(s, c)| format!("{c} {s}")).collect();
    out.push(format!("{} note{}: {}", notes.len(), if notes.len() == 1 { "" } else { "s" }, counts.join(", ")));
    if tally.contains_key("to restate") {
        out.push(take::RESTATE.into());
    }
    (out.join("\n"), code)
}

#[cfg(test)]
mod tests;
