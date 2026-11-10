use std::path::Path;

use raea_core_facts::Finding;

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
    assert!(out.contains("notes/1/n1.ts: round 1, to check: ") && out.contains("  n1.1 the lyric is flat type behind the singer: to check: no check, and no clip to sign"), "{out}");
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


fn calls() -> Vec<Vec<String>> {
    RAN.with(|r| r.borrow().clone())
}

#[test]
fn post_shows_each_note_on_its_comment_as_it_stands_once() {
    let (_tmp, p, inbox) = project();
    let whole = r#"{"item":"i-show-2","project":"shop","kind":"note","text":"hold the end card longer","when":"2026-11-10T11:00:00Z"}"#;
    std::fs::write(inbox.join("a.jsonl"), format!("{SENT}\n{whole}")).unwrap();
    take_from(&inbox, &p, &[]).unwrap();

    // taken and not restated: each comment's badge reads "to restate"
    let (out, code) = post::post(&p, eval, refused);
    assert_eq!((code, out.contains("notes/1/n1.ts: not posted: the raea app isn't running")), (1, true), "{out}");
    assert!(out.contains("notes/1/n3.ts: a note on its item as a whole has no comment to show its asks on"), "{out}");
    let (out, code) = post::post(&p, eval, ran);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1: to restate"), "{out}");
    assert_eq!(calls()[0], args(&["show", "--on", "i-show-1", "--working", "cm-1", "--status", "to restate"]));
    assert_eq!(calls().len(), 2, "cm-1 and cm-2");

    // one ask for two sentences still owes a reading: its asks aren't shown, and it still reads to restate
    restate_n1(&p, r#"{ "text": "the lyric is flat type", "medium": "2D", "not": ["3D text"] }"#);
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1 as it stands already (to restate)"), "{out}");
    assert_eq!(calls().len(), 2);

    // whole: its asks, the note to check (no check yet) and each ask's state
    restate_n1(&p, TWO);
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1: its 2 asks, to check"), "{out}");
    let want = args(&["show", "--on", "i-show-1", "--working", "cm-1", "--ask", "the lyric is flat type the singer passes in front of", "--ask", "the words arrive as a lyric video's do", "--status", "to check", "--states", "to check,to check"]);
    assert_eq!(calls()[2], want);
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1 as it stands already (to check)"), "{out}");
    assert_eq!(calls().len(), 3);

    // the person corrects ask 1: back to restate; restated again in the same words, it's shown again, as the answer
    let fix = r#"{"kind":"correction","item":"i-show-1","project":"shop","title":"Spring ad","comment":"cm-1","ask":1,"text":"yes, that"}"#;
    std::fs::write(inbox.join("a.jsonl"), format!("{SENT}\n{whole}\n{fix}")).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1: to restate: its restatement still owes: the person corrected ask 1"), "{out}");
    let f = p.join("notes/1/n1.ts");
    std::fs::write(&f, std::fs::read_to_string(&f).unwrap().replace("  corrected: [1],\n", "")).unwrap();
    post::post(&p, eval, ran);
    assert_eq!(calls().last().unwrap(), &want);
}

/// A finding of the ask `id`'s check, as a run gives it.
fn found(id: &str) -> Finding {
    Finding::new("faces", raea_core_facts::Level::Error, "desk-3 is turned 180 degrees from facing chair-3", &check::by(id))
}

/// One composition's run that found `fs`.
fn run_of(fs: Vec<Finding>) -> Vec<check::Ran> {
    vec![check::Ran { of: "office.tsx".into(), found: fs, waits: vec![], render: None, shots: vec![], fps: 30.0 }]
}

/// A project with n1 restated as two asks that run one check with different values, the second on one composition.
fn two_asks() -> (tempfile::TempDir, PathBuf) {
    let (tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    restate_n1(&p, r#"{ "text": "every desk faces its chair", "medium": "2D", "not": ["3D"], "check": "../../checks/faces.ts", "values": { "what": "desk" } }, { "text": "every chair faces its desk", "medium": "2D", "not": [], "of": "office.tsx", "check": "../../checks/faces.ts", "values": { "what": "chair" } }"#);
    (tmp, p)
}

#[test]
fn an_asks_check_must_fail_first_and_one_that_passes_first_is_refused() {
    let (_tmp, p) = two_asks();
    let notes = read(&p, eval);
    let (rules, bad) = check::rules(&p, &notes);
    assert!(bad.is_empty());
    assert_eq!(rules.iter().map(|r| (r.id.as_str(), r.by.as_str(), r.of.as_deref())).collect::<Vec<_>>(), [("n1.1", "note n1.1", None), ("n1.2", "note n1.2", Some("office.tsx"))]);
    assert_eq!(rules[0].check, p.join("notes/1/../../checks/faces.ts").display().to_string(), "a check's path is from the note's folder");
    let ask = |s: &state::State, i: usize| state::of_note(notes[0].note.as_ref().unwrap(), 1, s).1[i].clone();

    // the first run passes n1.1 (refused) and fails n1.2; each counts only its own findings
    let mut s = state::State::new();
    let (lines, refused) = check::settle(&p, &notes, &rules, &run_of(vec![found("n1.2"), found("n1.2")]), "20261110-120000", &mut s);
    assert!(refused);
    assert!(lines[0].contains("n1.1 notes/1/n1.ts: \"every desk faces its chair\": refused: its check passes on the work as it stands"), "{lines:?}");
    assert!(lines[1].contains("n1.2 notes/1/n1.ts: \"every chair faces its desk\": fails on the work as it stands (2 findings)"), "{lines:?}");
    assert!(matches!(ask(&s, 0), state::Standing::ToCheck(why) if why.contains("passed on its first run")));
    assert_eq!(ask(&s, 1), state::Standing::Failing(2));

    // n1.1 shown failing later counts; the work changes and both pass
    check::settle(&p, &notes, &rules, &run_of(vec![found("n1.1")]), "20261110-120100", &mut s);
    assert_eq!((ask(&s, 0), ask(&s, 1)), (state::Standing::Failing(1), state::Standing::Passing));
    let (lines, refused) = check::settle(&p, &notes, &rules, &run_of(vec![]), "20261110-120200", &mut s);
    assert!(!refused && lines.iter().all(|l| l.ends_with(": passing")), "{lines:?}");

    // --only runs one: the other isn't judged on a run it wasn't in
    let (lines, _) = check::settle(&p, &notes, &rules[1..], &run_of(vec![found("n1.2")]), "20261110-120300", &mut s);
    assert_eq!(lines.len(), 1);
    assert_eq!((ask(&s, 0), ask(&s, 1)), (state::Standing::Passing, state::Standing::Failing(1)));

    // a check changed after it failed is to be shown failing again; kept as written, ls says each ask's state
    let f = p.join("notes/1/n1.ts");
    std::fs::write(&f, std::fs::read_to_string(&f).unwrap().replace(r#""what": "desk""#, r#""what": "desk", "within": 90"#)).unwrap();
    let notes = read(&p, eval);
    assert!(matches!(state::of_note(notes[0].note.as_ref().unwrap(), 1, &s).1[0].clone(), state::Standing::ToCheck(why) if why.contains("changed since it failed")));
    state::write(&p, &s).unwrap();
    let (out, _) = ls(&p, false, eval);
    assert!(out.contains("  n1.2 every chair faces its desk [2D]: failing (1 finding)"), "{out}");
    assert!(out.contains("notes/1/n1.ts: round 1, to check: "), "{out}");
}

#[test]
fn a_clip_is_signed_by_the_person_for_its_render_and_reopens_when_its_source_changes() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    restate_n1(&p, r#"{ "text": "the light turns on the cut", "medium": "2D", "not": ["3D"], "proof": { "shot": "reveal", "sign": true } }, { "text": "the lyric is flat type", "medium": "2D", "not": [] }"#);
    let note = || read(&p, eval).remove(0).note.unwrap();
    let ask = |s: &state::State| state::of_note(&note(), 1, s).1[0].clone();
    let mut s = state::State::new();
    assert!(matches!(ask(&s), state::Standing::ToCheck(why) if why.starts_with("no clip yet")), "the agent's move: render and cut it");
    let clip = |render: &str, stale: bool| state::Clip { file: p.join(".raea/notes-clips/n1.1.mp4"), of: "edit.tsx".into(), render: render.into(), frames: [30, 150], stale };
    s.insert("n1.1".into(), state::Kept { clip: Some(clip("aaa", false)), ..Default::default() });
    state::write(&p, &s).unwrap();
    assert!(matches!(ask(&s), state::Standing::ToSign(why) if why.contains("waits on the person")));

    // posted under its ask, for the person to sign
    let (out, _) = post::post(&p, eval, ran);
    assert!(out.contains("notes/1/n1.ts: shown on cm-1 of i-show-1: its 2 asks, to check, with 1 clip to sign"), "{out}");
    let file = p.join(".raea/notes-clips/n1.1.mp4").display().to_string();
    assert_eq!(calls()[1], args(&["show", "--on", "i-show-1", "--working", "cm-1", "--sign", "1", "--clip", &file, "--render", "aaa"]), "after cm-1's asks, before cm-2");

    // the person signs it in the app (a sign for a note not taken is said, not dropped); a second take changes nothing
    let sign = |render: &str, name: &str| format!(r#"{{"kind":"sign","item":"i-show-1","project":"shop","comment":"cm-1","ask":1,"render":"{render}",{name}"when":"2026-11-10T12:00:00Z"}}"#);
    let lost = r#"{"kind":"sign","item":"i-show-7","project":"shop","comment":"cm-1","ask":1,"render":"aaa","when":"x"}"#;
    std::fs::write(inbox.join("a.jsonl"), [SENT.to_string(), sign("aaa", r#""name":"Sam","#), lost.to_string()].join("\n")).unwrap();
    let out = take_from(&inbox, &p, &[]).unwrap();
    assert!(out.contains("notes/1/n1.ts: ask 1 signed by Sam, on the render aaa…"), "{out}");
    assert!(out.contains("a sign for a note not in notes/ (app i-show-7 cm-1, ask 1)"), "{out}");
    let s = state::read(&p);
    assert_eq!(ask(&s), state::Standing::Signed(Some("Sam".into())));
    assert!(!take_from(&inbox, &p, &[]).unwrap().contains("signed"));

    // its composition changed after the render: to sign again; a new render's clip: the old sign doesn't hold
    let mut s = state::read(&p);
    s.get_mut("n1.1").unwrap().clip = Some(clip("aaa", true));
    assert!(matches!(ask(&s), state::Standing::ToCheck(why) if why.contains("changed after the render")), "the agent's move: render it again");
    s.get_mut("n1.1").unwrap().clip = Some(clip("bbb", false));
    assert!(matches!(ask(&s), state::Standing::ToSign(why) if why.starts_with("Sam signed an older render")));
    state::write(&p, &s).unwrap();

    // signed again for the new render, by a person the app had no account for: signed, unnamed
    std::fs::write(inbox.join("a.jsonl"), [SENT.to_string(), sign("aaa", r#""name":"Sam","#), sign("bbb", "")].join("\n")).unwrap();
    let out = take_from(&inbox, &p, &[]).unwrap();
    assert!(out.contains("ask 1 signed by the person (unnamed: the app had no account), on the render bbb…"), "{out}");
    assert_eq!(ask(&state::read(&p)), state::Standing::Signed(None));
}

#[test]
fn an_ask_names_its_composition_by_file_or_by_file_and_id() {
    assert!(check::is_of("hold.tsx", "hold.tsx#hold"), "a file alone is any composition in it");
    assert!(check::is_of("hold.tsx", "hold.tsx"));
    assert!(check::is_of("cuts.tsx#b", "cuts.tsx#b"));
    assert!(!check::is_of("cuts.tsx#b", "cuts.tsx#a"));
    assert!(!check::is_of("cuts.tsx#b", "cuts.tsx"), "an id the run doesn't name");
    assert!(!check::is_of("hold.tsx", "scenes/hold.tsx#hold"), "its file from the project, not its name");
}

#[test]
fn no_clip_is_cut_from_a_render_the_composition_changed_after() {
    let (_tmp, p, inbox) = project();
    std::fs::write(inbox.join("a.jsonl"), SENT).unwrap();
    take_from(&inbox, &p, &[]).unwrap();
    restate_n1(&p, r#"{ "text": "the light turns on the cut", "medium": "2D", "not": ["3D"], "of": "edit.tsx", "proof": { "shot": "reveal", "sign": true } }, { "text": "the lyric is flat type", "medium": "2D", "not": [] }"#);
    let notes = read(&p, eval);
    let mut s = state::State::new();
    let clip = state::Clip { file: p.join(".raea/notes-clips/n1.1.mp4"), of: "edit.tsx#edit".into(), render: "aaa".into(), frames: [30, 150], stale: false };
    s.insert("n1.1".into(), state::Kept { clip: Some(clip), ..Default::default() });
    let run = |render: Option<check::Rendered>| vec![check::Ran { of: "edit.tsx#edit".into(), found: vec![], waits: vec![], render, shots: vec![("reveal".into(), 30, None)], fps: 30.0 }];

    let stale = check::Rendered { out: p.join("out/edit.mp4"), stale: true, frames: 150 };
    let lines = check::clips(&p, &notes, &run(Some(stale)), &mut s);
    assert_eq!(lines, ["n1.1 notes/1/n1.ts: its clip waits for a render of edit.tsx#edit as it stands: it changed after its newest render"]);
    assert!(s["n1.1"].clip.as_ref().unwrap().stale, "the clip kept from the render before is marked");
    assert!(!p.join(".raea/notes-clips/n1.1.mp4").exists(), "nothing is cut from it");
    let n = notes[0].note.as_ref().unwrap();
    assert!(matches!(state::of_note(n, 1, &s).1[0].clone(), state::Standing::ToCheck(why) if why.contains("render it again")), "the agent's move");

    // no render at all: said, and nothing is cut
    assert_eq!(check::clips(&p, &notes, &run(None), &mut s), ["n1.1 notes/1/n1.ts: its clip waits for a render of edit.tsx#edit"]);
}
