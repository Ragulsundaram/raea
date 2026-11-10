use std::path::Path;

use super::*;
use crate::take::take_from;

/// A note file's object as JSON, for a test without raea's TS host: its keys quoted and a comma before a close
/// dropped (the files take writes are in that form).
fn eval(f: &Path) -> Result<String, String> {
    let t = std::fs::read_to_string(f).map_err(|e| e.to_string())?;
    let body = t.split_once("note(").ok_or("no note(")?.1.rsplit_once(");").ok_or("no );")?.0;
    let line = |l: &str| match l.trim().split_once(':') {
        Some((k, v)) if !k.is_empty() && k.chars().all(char::is_alphanumeric) => format!("\"{k}\":{v}"),
        _ => l.trim().to_string(),
    };
    Ok(body.lines().map(line).collect::<String>().replace(",}", "}").replace(",]", "]"))
}

/// A project folder named `shop`, and an inbox folder beside it.
fn project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("shop");
    std::fs::create_dir_all(p.join(".raea")).unwrap();
    std::fs::write(p.join(".raea/project"), "").unwrap();
    let inbox = tmp.path().join("inbox");
    std::fs::create_dir_all(&inbox).unwrap();
    (tmp, p, inbox)
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

const SENT: &str = r#"{"item":"i-show-1","project":"shop","title":"Spring ad","kind":"edits","text":"1. [0:04.5] the lamp","when":"2026-11-10T10:00:00Z","comments":[{"id":"cm-1","text":"I want 2D text with occlusion, not 3D. Think of a lyric video.","kind":"edit","pin":{"kind":"place","x":0.57,"y":0.67,"t":4.5},"at":"0:04.5 · 57%, 67%"},{"id":"cm-2","text":"the \"jug\" isn't visible","kind":"edit"}]}
{"item":"i-show-9","project":"other","kind":"note","text":"not this project's","when":"2026-11-10T10:01:00Z"}"#;

#[test]
fn what_the_person_sends_or_says_is_taken_once_as_notes_to_restate() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("claude-code-1.jsonl"), SENT).unwrap();
    let out = take_from(&inbox, &p, &args(&["--said", "redo the intro from scratch"])).unwrap();
    assert!(out.contains("notes/1/n1.ts: to restate: \"I want 2D text with occlusion, not 3D. Think of a lyric video.\" (at 0:04.5 · 57%, 67%)"), "{out}");
    assert!(out.contains("notes/1/n3.ts: to restate, a redesign: \"redo the intro from scratch\""), "{out}");
    assert!(out.contains("round 1: 3 notes taken."), "{out}");
    let notes = read(&p, eval);
    let n: Vec<&Note> = notes.iter().map(|k| k.note.as_ref().unwrap()).collect();
    assert_eq!(n[0].words, ["I want 2D text with occlusion, not 3D. Think of a lyric video."]);
    assert_eq!((n[0].from.as_str(), n[0].round), ("app i-show-1 cm-1", 1));
    assert_eq!(n[0].pin.as_ref().map(|p| (p.t, p.part.clone())), Some((Some(4.5), None)));
    assert_eq!(n[1].words, ["the \"jug\" isn't visible"], "words kept as written");
    assert_eq!((n[2].from.as_str(), n[2].redesign), ("session", Some(true)));
    assert!(!std::fs::read_to_string(p.join("notes/1/n1.ts")).unwrap().contains("null"), "a pin's missing keys aren't written");

    // taken again: nothing new; a later send is the next round
    let again = take_from(&inbox, &p, &args(&["--said", "redo the intro from scratch"])).unwrap();
    assert!(again.starts_with("no new notes for shop"), "{again}");
    let later = r#"{"item":"i-show-2","project":"shop","kind":"note","text":"hold the end card longer","when":"2026-11-10T11:00:00Z","pin":{"kind":"time","t":28.0}}"#;
    std::fs::write(inbox.join("claude-code-1.jsonl"), format!("{SENT}\n{later}")).unwrap();
    let out = take_from(&inbox, &p, &[]).unwrap();
    assert!(out.starts_with("notes/2/n4.ts: to restate: \"hold the end card longer\""), "{out}");
}

#[test]
fn ls_says_where_each_note_stands_and_what_it_owes() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    let (out, code) = ls(&p, false, eval);
    assert_eq!(code, 0);
    assert!(out.contains("notes/1/n1.ts: round 1, to restate: \"I want 2D text"), "{out}");
    assert!(out.ends_with(take::RESTATE), "{out}");
    assert!(out.contains("2 notes: 2 to restate"), "{out}");

    // restated as one ask with no medium: read again, and its 2D said
    let f = p.join("notes/1/n1.ts");
    let t = std::fs::read_to_string(&f).unwrap().replace("asks: [],", "asks: [{ \"text\": \"the lyric is flat type behind the singer\", \"not\": [] }],");
    std::fs::write(&f, t).unwrap();
    std::fs::write(p.join("notes/1/n9.ts"), "export default 3;").unwrap();
    let (out, code) = ls(&p, false, eval);
    assert_eq!(code, 1, "a note that can't be read");
    assert!(out.contains("notes/1/n1.ts: round 1, to check: ") && out.contains("  - the lyric is flat type behind the singer"), "{out}");
    assert!(out.contains("  ! one ask from 2 sentences: read it again for what else it asks"), "{out}");
    assert!(out.contains("  ! its words say 2D and no ask says it, as its medium or in its `not`") && out.contains("  ! its words say 3D and no ask says it"), "{out}");
    assert!(out.contains("notes/1/n9.ts: can't be read"), "{out}");
}

#[test]
fn notes_are_found_in_round_and_number_order_and_nothing_else_is_one() {
    let (_tmp, p, _) = project();
    for f in ["notes/2/n10.ts", "notes/2/n9.ts", "notes/1/n1.ts", "notes/1/check.ts", "notes/x/n2.ts"] {
        std::fs::create_dir_all(p.join(f).parent().unwrap()).unwrap();
        std::fs::write(p.join(f), "").unwrap();
    }
    let got: Vec<String> = files(&p).iter().map(|f| f.strip_prefix(&p).unwrap().display().to_string()).collect();
    assert_eq!(got, ["notes/1/n1.ts", "notes/2/n9.ts", "notes/2/n10.ts"]);
}

#[test]
fn a_note_is_an_entity_in_the_world() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    let mut w = Facts::new(0, 1920, 1080, 30.0);
    spawn(&mut w, &read(&p, eval));
    assert_eq!(w.find(&["Note"]).len(), 2);
}

/// n1 of SENT restated as two asks that owe nothing.
fn restate_n1(p: &Path, asks: &str) {
    let f = p.join("notes/1/n1.ts");
    let t = std::fs::read_to_string(&f).unwrap();
    let at = t.find("asks: [").unwrap();
    let end = at + t[at..].find("],\n").unwrap() + 2;
    std::fs::write(&f, format!("{}asks: [{asks}],{}", &t[..at], &t[end..])).unwrap();
}

const TWO: &str = r#"{ "text": "the lyric is flat type the singer passes in front of", "medium": "2D", "not": ["3D text"] }, { "text": "the words arrive as a lyric video's do", "medium": "2D", "not": [] }"#;

#[test]
fn a_correction_from_the_app_is_written_into_its_note_once_and_it_is_restated_again() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    restate_n1(&p, TWO);
    assert_eq!(read(&p, eval)[0].note.as_ref().unwrap().state(), "to check");

    let fix = |ask: &str, text: &str, comment: &str| format!(r#"{{"kind":"correction","item":"i-show-1","project":"shop","title":"Spring ad","comment":"{comment}",{ask}"text":"{text}"}}"#);
    let lines = [SENT.to_string(), fix(r#""ask":2,"#, "no, keep the font as it is", "cm-1"), fix("", "it's about the chorus only", "cm-1"), fix("", "lost", "cm-7")];
    std::fs::write(inbox.join("a.jsonl"), lines.join("\n")).unwrap();
    let out = take_from(&inbox, &p, &[]).unwrap();
    assert!(out.contains("notes/1/n1.ts: the person corrected ask 2: \"no, keep the font as it is\": restate it again"), "{out}");
    assert!(out.contains("notes/1/n1.ts: the person corrected the note: \"it's about the chorus only\": restate it again"), "{out}");
    assert!(out.contains("a correction for a note not in notes/ (app i-show-1 cm-7): \"lost\""), "{out}");
    assert!(!out.contains("no new notes"), "{out}");

    let n = read(&p, eval).remove(0).note.unwrap();
    assert_eq!(n.words, ["I want 2D text with occlusion, not 3D. Think of a lyric video.", "on ask 2: no, keep the font as it is", "it's about the chorus only"]);
    assert_eq!(n.corrected, Some(vec![0, 2]));
    assert_eq!(n.state(), "to restate");
    assert_eq!(n.asks.len(), 2, "the asks are kept for the agent to restate again");

    // taken again: each correction is in already
    let again = take_from(&inbox, &p, &[]).unwrap();
    assert!(!again.contains("corrected"), "{again}");
    assert_eq!(read(&p, eval).remove(0).note.unwrap().words.len(), 3);
}

thread_local! {
    static RAN: std::cell::RefCell<Vec<Vec<String>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn ran(a: &[String]) -> R<String> {
    RAN.with(|r| r.borrow_mut().push(a.to_vec()));
    Ok("Marked cm-1 working".into())
}

fn refused(_: &[String]) -> R<String> {
    Err("the raea app isn't running".into())
}

#[test]
fn post_shows_each_restated_note_on_its_comment_once_and_none_still_owing() {
    let (_tmp, p, inbox) = project();
    let whole = r#"{"item":"i-show-2","project":"shop","kind":"note","text":"hold the end card longer","when":"2026-11-10T11:00:00Z"}"#;
    std::fs::write(inbox.join("a.jsonl"), format!("{SENT}\n{whole}")).unwrap();
    take_from(&inbox, &p, &[]).unwrap();

    let (out, code) = post::post(&p, eval, refused);
    assert_eq!(code, 0, "nothing restated, nothing run: {out}");
    assert!(out.contains("notes/1/n1.ts: not posted: it's still to restate"), "{out}");
    assert!(out.contains("notes/1/n3.ts: a note on its item as a whole has no comment to show its asks on"), "{out}");

    // one ask for two sentences still owes a reading: not posted
    restate_n1(&p, r#"{ "text": "the lyric is flat type", "medium": "2D", "not": ["3D text"] }"#);
    let (out, _) = post::post(&p, eval, refused);
    assert!(out.contains("notes/1/n1.ts: not posted: its restatement still owes: one ask from 2 sentences"), "{out}");

    restate_n1(&p, TWO);
    let (out, code) = post::post(&p, eval, refused);
    assert_eq!((code, out.contains("notes/1/n1.ts: not posted: the raea app isn't running")), (1, true), "{out}");
    let (out, code) = post::post(&p, eval, ran);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("notes/1/n1.ts: its 2 asks shown on cm-1 of i-show-1, where the person can correct one"), "{out}");
    let want = args(&["show", "--on", "i-show-1", "--working", "cm-1", "--ask", "the lyric is flat type the singer passes in front of", "--ask", "the words arrive as a lyric video's do"]);
    assert_eq!(RAN.with(|r| r.borrow().clone()), [want]);

    // shown already: not again (a comment settled since isn't marked working again), until its asks change
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: its asks are shown on cm-1 of i-show-1 already"), "{out}");
    assert_eq!(RAN.with(|r| r.borrow().len()), 1);
    restate_n1(&p, &TWO.replace("as a lyric video's do", "one by one on the beat"));
    post::post(&p, eval, ran);
    assert_eq!(RAN.with(|r| r.borrow().len()), 2);

    // the person corrects ask 1; restated again in the same words, it's shown again, as the answer to their correction
    let fix = r#"{"kind":"correction","item":"i-show-1","project":"shop","title":"Spring ad","comment":"cm-1","ask":1,"text":"yes, that"}"#;
    std::fs::write(inbox.join("a.jsonl"), format!("{SENT}\n{whole}\n{fix}")).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    let f = p.join("notes/1/n1.ts");
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: not posted: its restatement still owes: the person corrected ask 1"), "{out}");
    std::fs::write(&f, std::fs::read_to_string(&f).unwrap().replace("  corrected: [1],\n", "")).unwrap();
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: its 2 asks shown on cm-1"), "{out}");
    assert_eq!(RAN.with(|r| r.borrow().len()), 3);
}
