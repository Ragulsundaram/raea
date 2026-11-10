//! `raea notes take`: what the person sent from the raea app to this project (the inbox lines it wrote: each edit
//! comment of a send, with its pin, or a note on an item) and what they said in a session (`--said`), each written
//! as a note file with their words verbatim and no asks, all in one new round. A note already taken isn't taken again.
//! What the person sends back on a restated note comes the same way: a correction, written into its note (DR5), and
//! a sign on an ask's clip, kept with what that ask has been through (state.rs, DR6).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use raea_core_facts::{Pin, note};
use serde_json::Value;

use crate::{R, files, place, quoted, state, values};

/// What a note asks of the agent before anything changes, said where notes are taken and listed.
pub(crate) const RESTATE: &str = "restate each note before changing anything: one ask for each thing it asks for, the medium where its words name one (2D, 3D, flat, extruded, real, modelled, voxel), what it is about, and what it doesn't ask for (`not`), so a change keeps the rest";

/// A note to write: the person's words, where they came from, its pin, and where it was as the app said it.
pub(crate) struct New {
    pub words: String,
    pub from: String,
    pub pin: Option<Pin>,
    pub at: Option<String>,
}

/// Where the raea app writes what the person sends (the app's host, inbox.ts).
fn inbox_dir() -> PathBuf {
    match std::env::var_os("RAEA_INBOX_DIR") {
        Some(dir) => dir.into(),
        None => Path::new(&std::env::var_os("HOME").unwrap_or_default()).join(".raea/inbox"),
    }
}

/// The app's pin ({kind: "place", x, y, t, part} or {kind: "time", t}) as a note's.
fn pin_of(v: &Value) -> Option<Pin> {
    v.is_object().then(|| Pin { t: v["t"].as_f64(), x: v["x"].as_f64(), y: v["y"].as_f64(), part: v["part"].as_str().map(Into::into) })
}

/// The notes in one inbox line sent to `project`: each comment of a send of edits, or a note on an item.
pub(crate) fn of_line(v: &Value, project: &str) -> Vec<New> {
    if v["project"].as_str() != Some(project) {
        return Vec::new();
    }
    let item = v["item"].as_str().unwrap_or("");
    let said = |v: &Value| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    match v["kind"].as_str() {
        // a send's own text is its comments listed again
        Some("edits") => v["comments"].as_array().into_iter().flatten().filter_map(|c| {
            Some(New { words: said(&c["text"])?, from: format!("app {item} {}", c["id"].as_str()?), pin: pin_of(&c["pin"]), at: said(&c["at"]) })
        }).collect(),
        Some("note") => said(&v["text"]).map(|words| New { words, from: format!("app {item} {}", v["when"].as_str().unwrap_or("")), pin: pin_of(&v["pin"]), at: None }).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// A correction the person made in the app to a restated note: the note (by where it came from), the ask (1 is the
/// first; none: the note as a whole), and their line.
pub(crate) struct Correction {
    pub from: String,
    pub ask: u32,
    pub text: String,
}

/// The correction in one inbox line sent to `project`, if it is one.
pub(crate) fn correction_of(v: &Value, project: &str) -> Option<Correction> {
    if v["project"].as_str() != Some(project) || v["kind"].as_str() != Some("correction") {
        return None;
    }
    let text = v["text"].as_str().map(str::trim).filter(|t| !t.is_empty())?;
    let ask = v["ask"].as_u64().and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
    Some(Correction { from: format!("app {} {}", v["item"].as_str()?, v["comment"].as_str()?), ask, text: text.into() })
}

/// Where the list that opens at `open` (its `[`) in `t` closes, past what is quoted in it.
fn close_of(t: &str, open: usize) -> Option<usize> {
    let (mut depth, mut quote, mut escaped) = (0, None, false);
    for (i, c) in t[open..].char_indices() {
        match (quote, c) {
            (Some(_), _) if escaped => escaped = false,
            (Some(_), '\\') => escaped = true,
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, '[') => depth += 1,
            (None, ']') => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// A note file's text with the person's correction added: their line last in its words, and its ask in `corrected`.
/// None when the file has no words list or asks to add them beside.
pub(crate) fn corrected(t: &str, c: &Correction) -> Option<String> {
    let line = if c.ask == 0 { c.text.clone() } else { format!("on ask {}: {}", c.ask, c.text) };
    let open = t.find("words: [")? + "words: ".len();
    let end = close_of(t, open)?;
    let mut out = format!("{}, {}{}", t[..end].trim_end().trim_end_matches(','), serde_json::to_string(&line).ok()?, &t[end..]);
    match out.find("corrected: [") {
        Some(at) => {
            let at = at + "corrected: [".len();
            let sep = if out[at..].starts_with(']') { "" } else { ", " };
            out.insert_str(at, &format!("{}{sep}", c.ask));
        }
        None => {
            // `asks:` where it starts its line, so a word in a quote is never taken for it
            let at = out.match_indices("asks:").map(|(i, _)| i).find(|&i| out[..i].rsplit('\n').next().is_some_and(|l| l.trim().is_empty()))?;
            let start = out[..at].rfind('\n').map_or(0, |i| i + 1);
            let indent = out[start..at].to_string();
            out.insert_str(start, &format!("{indent}corrected: [{}],\n", c.ask));
        }
    }
    Some(out)
}

/// The person's sign on an ask's clip in the app: the note (by where it came from), the ask (1 is the first), the
/// render they watched (its SHA-256), their name (none when the app had no account) and when.
pub(crate) struct Signed {
    pub from: String,
    pub ask: u32,
    pub render: String,
    pub name: Option<String>,
    pub when: String,
}

/// The sign in one inbox line sent to `project`, if it is one.
pub(crate) fn sign_of(v: &Value, project: &str) -> Option<Signed> {
    if v["project"].as_str() != Some(project) || v["kind"].as_str() != Some("sign") {
        return None;
    }
    let ask = v["ask"].as_u64().and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0)?;
    let name = v["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).map(String::from);
    Some(Signed { from: format!("app {} {}", v["item"].as_str()?, v["comment"].as_str()?), ask, render: v["render"].as_str()?.into(), name, when: v["when"].as_str().unwrap_or("").into() })
}

/// Each ask's last sign kept with what it has been through (state.rs), once: what was signed, or why it wasn't.
fn sign(project: &Path, kept: &[PathBuf], signs: Vec<Signed>) -> R<Vec<String>> {
    // the last sign of each ask: a newer render's sign replaces an older one's
    let mut last: BTreeMap<(String, u32), Signed> = BTreeMap::new();
    for g in signs {
        last.insert((g.from.clone(), g.ask), g);
    }
    let mut s = state::read(project);
    let before = s.clone();
    let mut out = Vec::new();
    for g in last.into_values() {
        let from = format!("from: {}", serde_json::to_string(&g.from).unwrap_or_default());
        // the note whose file says it came from this comment
        let note = kept.iter().find_map(|f| {
            let said = std::fs::read_to_string(f).ok()?;
            if said.contains(&from) { Some((f, place(project, f)?)) } else { None }
        });
        let Some((f, (_, number))) = note else {
            out.push(format!("a sign for a note not in notes/ ({}, ask {})", g.from, g.ask));
            continue;
        };
        let k = s.entry(state::id(number, g.ask as usize)).or_default();
        let signed = state::Sign { name: g.name.clone(), render: g.render.clone(), at: g.when.clone() };
        if k.signed.as_ref() == Some(&signed) {
            continue;
        }
        let now = k.clip.as_ref().is_some_and(|c| c.render == g.render);
        k.signed = Some(signed);
        let rel = f.strip_prefix(project).unwrap_or(f).display();
        let by = g.name.as_deref().unwrap_or("the person (unnamed: the app had no account)");
        let short: String = g.render.chars().take(12).collect();
        out.push(format!("{rel}: ask {} signed by {by}, on the render {short}…{}", g.ask, if now { "" } else { ", not the one its clip is cut from now: it is to sign again" }));
    }
    if s != before {
        state::write(project, &s)?;
    }
    Ok(out)
}

/// Each correction applied to its note's file, once: what was done, or why it wasn't.
fn apply(project: &Path, kept: &[PathBuf], corrections: &[Correction]) -> R<Vec<String>> {
    let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
    let mut out = Vec::new();
    for c in corrections {
        let line = if c.ask == 0 { c.text.clone() } else { format!("on ask {}: {}", c.ask, c.text) };
        let Some(f) = kept.iter().find(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains(&format!("from: {}", s(&c.from))))) else {
            out.push(format!("a correction for a note not in notes/ ({}): {}", c.from, quoted(&c.text, 90)));
            continue;
        };
        let t = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        if t.contains(&s(&line)) {
            continue;
        }
        let rel = f.strip_prefix(project).unwrap_or(f).display().to_string();
        match corrected(&t, c) {
            Some(new) => {
                std::fs::write(f, new).map_err(|e| format!("{}: {e}", f.display()))?;
                crate::post::forget(project, &c.from)?;
                let which = if c.ask == 0 { "the note".to_string() } else { format!("ask {}", c.ask) };
                out.push(format!("{rel}: the person corrected {which}: {}: restate it again", quoted(&c.text, 90)));
            }
            None => out.push(format!("{rel}: a correction can't be written in (no `words: [` and `asks:` to put it beside): {}", quoted(&c.text, 90))),
        }
    }
    Ok(out)
}

/// A note's file: its words and where they came from, its round, its pin, and no asks.
pub(crate) fn file_text(n: &New, round: u32) -> String {
    let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
    // what the pin doesn't say is left out, as the type has it (a key with no value isn't written as null)
    let pin = n.pin.as_ref().and_then(|p| serde_json::to_value(p).ok()).map(|mut p| {
        if let Some(o) = p.as_object_mut() {
            o.retain(|_, v| !v.is_null());
        }
        format!("  pin: {p},\n")
    });
    let redesign = if note::redesign(&n.words) { "  redesign: true,\n" } else { "" };
    let words: Vec<String> = n.words.lines().map(s).collect();
    format!(
        "// A note from the person, as they wrote it (raea notes take). Keep its words; restate it as asks below, one for each\n// thing it asks for, before anything in the work changes.\nimport {{ note }} from \"raea/note\";\n\nexport default note({{\n  words: [{}],\n  from: {},\n  round: {round},\n{pin}{redesign}  asks: [],\n}});\n",
        words.join(", "),
        s(&n.from),
        pin = pin.unwrap_or_default()
    )
}

pub(crate) fn take(project: &Path, args: &[String]) -> R<String> {
    take_from(&inbox_dir(), project, args)
}

/// `take`, reading the inbox files in `dir`.
pub(crate) fn take_from(dir: &Path, project: &Path, args: &[String]) -> R<String> {
    let name = project.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let inboxes: Vec<PathBuf> = match values(args, "--inbox").first() {
        Some(n) => vec![dir.join(format!("{}.jsonl", n.trim_end_matches(".jsonl")))],
        None => {
            let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jsonl")).collect();
            v.sort();
            v
        }
    };
    let (mut new, mut fixes, mut signs): (Vec<New>, Vec<Correction>, Vec<Signed>) = (Vec::new(), Vec::new(), Vec::new());
    for f in &inboxes {
        let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        for v in text.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()) {
            new.extend(of_line(&v, &name));
            fixes.extend(correction_of(&v, &name));
            signs.extend(sign_of(&v, &name));
        }
    }
    new.extend(values(args, "--said").into_iter().map(str::trim).filter(|w| !w.is_empty()).map(|w| New { words: w.into(), from: "session".into(), pin: None, at: None }));

    // a note already in notes/ (by where it came from; one said in a session, by its words too) isn't taken again
    let kept = files(project);
    let texts: Vec<String> = kept.iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect();
    let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
    let mut seen: Vec<(String, String)> = Vec::new();
    new.retain(|n| {
        let had = texts.iter().any(|t| t.contains(&format!("from: {}", s(&n.from))) && (n.from != "session" || n.words.lines().all(|l| t.contains(&s(l)))));
        let twice = seen.contains(&(n.from.clone(), n.words.clone()));
        seen.push((n.from.clone(), n.words.clone()));
        !had && !twice
    });
    let mut out = Vec::new();
    if !new.is_empty() {
        let at = kept.iter().filter_map(|f| place(project, f));
        let (round, mut last) = at.fold((0, 0), |(r, n), (fr, fnum)| (r.max(fr), n.max(fnum)));
        let round = round + 1;
        let folder = project.join("notes").join(round.to_string());
        std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        for n in &new {
            last += 1;
            let file = folder.join(format!("n{last}.ts"));
            std::fs::write(&file, file_text(n, round)).map_err(|e| format!("{}: {e}", file.display()))?;
            let at = n.at.as_deref().map(|a| format!(" (at {a})")).unwrap_or_default();
            let redesign = if note::redesign(&n.words) { ", a redesign" } else { "" };
            out.push(format!("notes/{round}/n{last}.ts: to restate{redesign}: {}{at}", quoted(&n.words, 90)));
        }
        out.push(format!("round {round}: {} note{} taken. {RESTATE}", new.len(), if new.len() == 1 { "" } else { "s" }));
    }
    // corrections last, so one for a note taken just now finds its file
    out.extend(apply(project, &files(project), &fixes)?);
    out.extend(sign(project, &files(project), signs)?);
    if out.is_empty() {
        return Ok(format!("no new notes for {name}: what was sent to it from the raea app ({} inbox file{}) or said is in notes/ already (raea notes ls)", inboxes.len(), if inboxes.len() == 1 { "" } else { "s" }));
    }
    Ok(out.join("\n"))
}
