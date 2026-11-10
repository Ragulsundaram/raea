//! A person's notes on a project (docs/direction.feature DR5, DR6). `raea notes take` writes each note the person
//! sends from the raea app (an edit comment with its pin, or a note on an item) or says in a session to a file,
//! notes/<round>/<id>.ts, with their words verbatim and no asks. The agent restates each as its asks before anything
//! in the work changes, each with a check, or a clip for the person to sign. `raea notes check` runs each ask's check
//! on the work as it stands, which must fail first, and `raea notes ls` says where each note and ask stands and what
//! its restatement still owes. `raea notes post` shows the asks and where they stand on the person's comments in the
//! app, and their corrections and signs come back with the next take. In a project's world each note is an entity
//! with a Note.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use raea_core_facts::{Facts, Note, record};
use serde_json::json;

pub mod check;
mod post;
pub mod state;
mod take;

pub const USAGE: &str = "raea notes take [PROJECT] [--inbox NAME] [--said WORDS]... | raea notes check [PROJECT] [--composition ID] [--render OUT | --no-render] | raea notes ls [PROJECT] [--json] | raea notes post [PROJECT]   (a person's notes as files the work is held to: take writes each note sent from the raea app or said in a session to notes/ROUND/ID.ts, the words verbatim, to be restated as asks before anything changes, and writes in each correction and sign the person sent back; check runs each ask's check on the work as it stands, which must fail before the work changes (a check that passes first is refused), and cuts each clip the person is to sign from the newest render; ls says where each note and ask stands (to restate, to check, failing, to sign, passing, signed) and what its restatement still owes; post shows each note's asks and where they stand on its comment in the raea app, and each clip to sign)";

/// Evaluates a .ts file's default export to its JSON (the CLI's spec host).
pub type Eval = fn(&Path) -> Result<String, String>;

pub(crate) type R<T> = Result<T, String>;

/// The flags `raea notes` takes, and whether each takes a value.
const FLAGS: [(&str, bool); 6] = [("--inbox", true), ("--said", true), ("--json", false), ("--composition", true), ("--render", true), ("--no-render", false)];

/// One note file: where it is in the project, and the note, or why it can't be read.
pub struct Kept {
    pub file: PathBuf,
    pub note: R<Note>,
}

/// `raea notes VERB ...`, printed; its exit code (1: a note that can't be read, or an ask's check refused; 2: the
/// command was wrong). `run` is `raea check`'s run of rules on the project's compositions (apps/direction).
pub fn cli(args: &[String], eval: Eval, run: check::Runner) -> i32 {
    match go(args, eval, run) {
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

fn go(args: &[String], eval: Eval, run: check::Runner) -> R<(String, i32)> {
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
        "check" => check::check(&project, rest, eval, run),
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

/// Each note's number (its file's: n7 is 7), where it stands and each of its asks'.
fn standings(project: &Path, n: &Note, file: &Path, s: &state::State) -> (u32, state::Standing, Vec<state::Standing>) {
    let number = place(project, &project.join(file)).map_or(0, |p| p.1);
    let (note, asks) = state::of_note(n, number, s);
    (number, note, asks)
}

/// How many notes stand where: "5 notes: 2 failing, 1 to restate".
pub(crate) fn tally(project: &Path, notes: &[Kept], s: &state::State) -> String {
    let mut t: BTreeMap<&str, usize> = BTreeMap::new();
    for k in notes {
        *t.entry(k.note.as_ref().map_or("can't be read", |n| standings(project, n, &k.file, s).1.word())).or_default() += 1;
    }
    let counts: Vec<String> = t.iter().map(|(s, c)| format!("{c} {s}")).collect();
    format!("{} note{}: {}", notes.len(), if notes.len() == 1 { "" } else { "s" }, counts.join(", "))
}

/// `raea notes ls`: each note with where it stands, each ask with its id and where it stands, and what its
/// restatement still owes; 1 when a note can't be read.
fn ls(project: &Path, as_json: bool, eval: Eval) -> (String, i32) {
    let notes = read(project, eval);
    let s = state::read(project);
    let code = i32::from(notes.iter().any(|k| k.note.is_err()));
    if as_json {
        let v: Vec<_> = notes
            .iter()
            .map(|k| match &k.note {
                Ok(n) => {
                    let (number, note, asks) = standings(project, n, &k.file, &s);
                    let each: Vec<_> = asks.iter().enumerate().map(|(i, a)| json!({ "id": state::id(number, i + 1), "state": a.word(), "says": a.said() })).collect();
                    json!({ "file": k.file, "note": n, "state": note.word(), "asks": each, "flags": n.flags() })
                }
                Err(e) => json!({ "file": k.file, "error": e }),
            })
            .collect();
        return (serde_json::to_string_pretty(&v).unwrap_or_default(), code);
    }
    if notes.is_empty() {
        return ("no notes yet: `raea notes take` writes each one the person sends from the raea app or says".into(), 0);
    }
    let mut out = Vec::new();
    let mut restate = false;
    for k in &notes {
        let file = k.file.display();
        let Ok(n) = &k.note else {
            out.push(format!("{file}: can't be read: {}", k.note.as_ref().err().map_or("", String::as_str)));
            continue;
        };
        let (number, note, asks) = standings(project, n, &k.file, &s);
        restate |= note == state::Standing::ToRestate;
        out.push(format!("{file}: round {}, {}: {}", n.round, note.word(), quoted(n.words.first().map_or("", String::as_str), 90)));
        for (i, (a, st)) in n.asks.iter().zip(&asks).enumerate() {
            let medium = a.medium.as_deref().map(|m| format!(" [{m}]")).unwrap_or_default();
            let not = if a.not.is_empty() { String::new() } else { format!("; not: {}", a.not.join(", ")) };
            out.push(format!("  {} {}{medium}{not}: {}", state::id(number, i + 1), a.text, st.said()));
        }
        out.extend(n.flags().into_iter().map(|f| format!("  ! {f}")));
    }
    out.push(tally(project, &notes, &s));
    if restate {
        out.push(take::RESTATE.into());
    }
    (out.join("\n"), code)
}

#[cfg(test)]
mod tests;
