//! Notes become evidence (docs/fingerprint.feature FP5a, FP5b). `raea fingerprint read PROJECT [--by kimi|claude]`
//! has a fresh agent session read each pinned note on the project's videos into a kind (defect, direction, praise or
//! both) and an axis from the fingerprint's own axes, kept in the fingerprint's readings.jsonl. Nothing trains on them
//! yet: `raea fingerprint readings --ask N` puts N readings to the person as questions in the app (`raea show
//! --question`, answers to the "fingerprint" inbox), `readings --tally` counts how often the person agreed, and only
//! once the kind is right 45 times in 50 and the axis 40 (FP5b) does `raea fingerprint learn-notes` make each defect
//! or praise an evidence row over its span (1.5 s before the pin to 0.5 s after). A direction goes to the brief and the
//! agent, never into a head. A note's marks (FP11b) make its row a region label: the box around its marks, boxes and
//! lassos; a span pin sets the row's time.

use crate::{flag, has, words};
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint::taxonomy::Fingerprint;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// FP5b's bar, out of every 50 checked
pub const KIND_BAR: f64 = 45.0 / 50.0;
pub const AXIS_BAR: f64 = 40.0 / 50.0;
pub const INBOX: &str = "fingerprint";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reading {
    pub note: String,
    pub text: String,
    pub who: String,
    pub media: PathBuf,
    pub t: f64,
    pub project: String,
    /// defect, direction, praise or both
    pub kind: String,
    pub axis: Option<String>,
    /// how it was read: kimi or claude
    pub by: String,
    /// the person's check: Some(true) agreed, Some(false) not; None until asked
    #[serde(default)]
    pub kind_ok: Option<bool>,
    #[serde(default)]
    pub axis_ok: Option<bool>,
    /// a span pin's end: the label covers t..to (FP11b)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<f64>,
    /// where in the frame the note's marks say the thing is, x0,y0,x1,y1 in 0-1 (FP11b)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<[f32; 4]>,
}

/// How far a click reaches around its point, as a region, until a click is grown into its thing (FP11b, EdgeTAM)
const CLICK: f32 = 0.05;

/// The region a note's marks name: the box around its marks, boxes and lassos (a click as a small square around it).
/// Arrows and strikes are instructions for the agent, never a region
pub fn region(pin: &Value) -> Option<[f32; 4]> {
    let mut r: Option<[f32; 4]> = None;
    for m in pin["marks"].as_array().into_iter().flatten() {
        let reach = match m["kind"].as_str() {
            Some("mark") => CLICK,
            Some("box" | "lasso") => 0.0,
            _ => continue,
        };
        for p in m["points"].as_array().into_iter().flatten() {
            let (Some(x), Some(y)) = (p[0].as_f64(), p[1].as_f64()) else { continue };
            let (x, y) = (x as f32, y as f32);
            let b = [(x - reach).max(0.0), (y - reach).max(0.0), (x + reach).min(1.0), (y + reach).min(1.0)];
            r = Some(r.map_or(b, |a| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]));
        }
    }
    r.filter(|a| a[2] > a[0] && a[3] > a[1])
}

/// A post's video on this machine: the renders library, else the project's own folder
fn media(project: &Path, m: &Value) -> Option<PathBuf> {
    let p = m["path"].as_str()?;
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    [home.join(".raea/feed/media").join(p), project.join(p)].into_iter().find(|f| f.is_file())
}

/// Every video in the project's feed, each version's, found on this machine (FP7a's postmortem looks back on them)
pub fn videos(project: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(project.join(".raea/feed")).into_iter().flatten().flatten() {
        let Some(post) = std::fs::read_to_string(e.path()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
        for m in post["media"].as_array().into_iter().flatten().filter(|m| m["kind"] == "video") {
            if let Some(v) = media(project, m).filter(|v| !out.contains(v)) {
                out.push(v);
            }
        }
    }
    out.sort();
    out
}

/// Every person's pinned note on a video in the project's feed
pub fn pinned(project: &Path) -> Vec<Reading> {
    let name = project.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut out = Vec::new();
    for e in std::fs::read_dir(project.join(".raea/feed")).into_iter().flatten().flatten() {
        let Ok(post) = std::fs::read_to_string(e.path()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).ok_or(()) else { continue };
        let Some(video) = post["media"].as_array().into_iter().flatten().find(|m| m["kind"] == "video").and_then(|m| media(project, m)) else { continue };
        for n in post["notes"].as_array().into_iter().flatten() {
            let (Some(t), Some(text)) = (n["pin"]["t"].as_f64(), n["text"].as_str()) else { continue };
            if n["author"]["kind"] != "person" {
                continue;
            }
            let to = n["pin"]["to"].as_f64().filter(|&to| to > t);
            out.push(Reading { note: n["id"].as_str().unwrap_or("").into(), text: text.into(), who: n["author"]["name"].as_str().unwrap_or("me").into(), media: video.clone(), t, project: name.clone(), to, region: region(&n["pin"]), ..Default::default() });
        }
    }
    out
}

/// The reading prompt: the axes with their words, then the notes, and the JSON to answer
pub fn prompt(fp: &Fingerprint, notes: &[Reading]) -> String {
    let axes = fp.axes.iter().map(|a| format!("- {}: {}", a.name, a.words.join("; "))).collect::<Vec<_>>().join("\n");
    let list = notes.iter().map(|n| format!("{}: {}", n.note, n.text.replace('\n', " "))).collect::<Vec<_>>().join("\n");
    format!(
        "You read a person's review notes on their videos. Read each note only; open no file and change nothing.\n\
For each note say its kind: \"defect\" (it names something wrong at that moment), \"direction\" (it asks for a creative change, \
not a fault), \"praise\" (it says something is good) or \"both\" (a defect and a direction). Then the axis it is about, from \
these and only these, or null when none fits:\n{axes}\n\nThe notes, one a line, id first:\n{list}\n\n\
End with only this JSON, nothing after it: [{{\"id\": \"\", \"kind\": \"defect|direction|praise|both\", \"axis\": \"name or null\"}}]"
    )
}

/// The JSON array an answer ends with
pub fn answer(text: &str) -> Option<Vec<Value>> {
    let end = text.rfind(']')?;
    let start = text[..end].rfind("[{").or_else(|| text[..end].rfind('['))?;
    serde_json::from_str(&text[start..=end]).ok()
}

/// Fold an answer into the notes: a kind it doesn't know or an axis not in the fingerprint is dropped
pub fn fold(notes: &mut [Reading], fp: &Fingerprint, ans: &[Value], by: &str) {
    for a in ans {
        let Some(id) = a["id"].as_str() else { continue };
        let Some(n) = notes.iter_mut().find(|n| n.note == id) else { continue };
        let kind = a["kind"].as_str().unwrap_or("");
        if !matches!(kind, "defect" | "direction" | "praise" | "both") {
            continue;
        }
        n.kind = kind.into();
        n.axis = a["axis"].as_str().filter(|x| fp.axis(x).is_some()).map(str::to_string);
        n.by = by.into();
    }
}

/// One fresh session's words: kimi's or claude's stream, the assistant's text joined
fn ask(by: &str, p: &str) -> Result<String, String> {
    let mut c = std::process::Command::new(by);
    match by {
        "kimi" => c.args(["-p", p, "--output-format", "stream-json"]),
        "claude" => c.args(["-p", p, "--output-format", "stream-json", "--verbose", "--allowedTools", ""]),
        o => return Err(format!("--by {o}: kimi or claude")),
    };
    let out = c.output().map_err(|e| format!("couldn't run {by}: {e}"))?;
    let mut text = String::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let content = if v["role"] == "assistant" { &v["content"] } else if v["type"] == "assistant" { &v["message"]["content"] } else if v["type"] == "result" { &v["result"] } else { continue };
        match content {
            Value::String(s) => text += s,
            Value::Array(a) => a.iter().filter_map(|p| p["text"].as_str()).for_each(|s| text += s),
            _ => {}
        }
        text.push('\n');
    }
    if text.trim().is_empty() {
        return Err(format!("{by} said nothing: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    Ok(text)
}

pub fn readings_of(dir: &Folder) -> Vec<Reading> {
    std::fs::read_to_string(dir.dir.join("readings.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

pub fn write_readings(dir: &Folder, r: &[Reading]) -> std::io::Result<()> {
    std::fs::write(dir.dir.join("readings.jsonl"), r.iter().filter_map(|x| serde_json::to_string(x).ok()).map(|l| l + "\n").collect::<String>())
}

/// `raea fingerprint read PROJECT [--by kimi|claude]`
pub(crate) fn read_here(args: &[String], cwd: &Path) -> i32 {
    let run = || -> Result<String, String> {
        let w = words(args)?;
        let project = w.first().map_or(cwd.to_path_buf(), |p| cwd.join(p));
        let by = flag(args, "--by").unwrap_or("kimi");
        let dir = crate::pick::chosen(args, Some(&crate::pick::piece_in(args, cwd))).0;
        let fp = dir.read(None).map_err(|e| e.to_string())?;
        let mut notes = pinned(&project);
        let mut kept = readings_of(&dir);
        notes.retain(|n| !kept.iter().any(|k| k.note == n.note && k.media == n.media));
        if notes.is_empty() {
            return Ok(format!("no new pinned notes in {}", project.display()));
        }
        for chunk in notes.chunks_mut(40) {
            let text = ask(by, &prompt(&fp, chunk))?;
            let ans = answer(&text).ok_or_else(|| format!("{by} answered no readings JSON"))?;
            fold(chunk, &fp, &ans, by);
            eprintln!("read {} notes", chunk.len());
        }
        let read = notes.iter().filter(|n| !n.kind.is_empty()).count();
        kept.extend(notes.into_iter().filter(|n| !n.kind.is_empty()));
        write_readings(&dir, &kept).map_err(|e| e.to_string())?;
        Ok(format!("{read} notes read into labels by {by}, kept in {}; none trains a head until you've checked them: raea fingerprint readings --ask 50", dir.dir.join("readings.jsonl").display()))
    };
    match run() {
        Ok(o) => {
            println!("{o}");
            0
        }
        Err(e) => {
            eprintln!("raea fingerprint read: {e}");
            1
        }
    }
}

/// The share of checked readings the person agreed with, for kind and for axis, and how many were checked
pub fn agreement(r: &[Reading]) -> (Option<f64>, Option<f64>, usize) {
    let share = |v: Vec<bool>| (!v.is_empty()).then(|| v.iter().filter(|x| **x).count() as f64 / v.len() as f64);
    let checked = r.iter().filter(|x| x.kind_ok.is_some()).count();
    (share(r.iter().filter_map(|x| x.kind_ok).collect()), share(r.iter().filter_map(|x| x.axis_ok).collect()), checked)
}

/// Whether the readings have earned training (FP5b): 50 checked, the kind right 45 times and the axis 40
pub fn earned(r: &[Reading]) -> bool {
    matches!(agreement(r), (Some(k), Some(a), n) if n >= 50 && k >= KIND_BAR && a >= AXIS_BAR)
}

/// The choices a reading is checked with, and what each says about kind and axis
pub const CHOICES: [(&str, bool, bool); 4] = [("Right", true, true), ("Right kind, wrong axis", true, false), ("Wrong kind", false, false), ("Right axis, wrong kind", false, true)];

/// `raea fingerprint readings [PROJECT] [--ask N] [--tally]`: the readings kept in the fingerprint that judges PROJECT
pub(crate) fn readings_here(args: &[String], cwd: &Path) -> i32 {
    let dir = crate::pick::chosen(args, Some(&crate::pick::piece_in(args, cwd))).0;
    let mut r = readings_of(&dir);
    if let Some(n) = flag(args, "--ask").and_then(|n| n.parse::<usize>().ok()) {
        let todo: Vec<usize> = r.iter().enumerate().filter(|(_, x)| x.kind_ok.is_none()).map(|(i, _)| i).take(n).collect();
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap_or_else(|_| "raea".into()));
        cmd.args(["show", "--title", "Check how raea read your notes", "--inbox", INBOX]);
        for &i in &todo {
            let x = &r[i];
            let q = format!("[{}] \"{}\" read as {} on {}", x.note, x.text.replace('\n', " "), x.kind, x.axis.as_deref().unwrap_or("no axis"));
            cmd.args(["--question", &q, "--preview", &x.media.display().to_string()]);
            for (c, _, _) in CHOICES {
                cmd.args(["--choice", c]);
            }
        }
        if todo.is_empty() {
            println!("every reading is checked");
            return 0;
        }
        let Ok(out) = cmd.output() else { return 1 };
        let said = String::from_utf8_lossy(&out.stdout).into_owned();
        // the item raea show made, so the answers can be matched to the notes asked, in order
        let Some(item) = said.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',').find(|w| w.starts_with("i-show-")) else {
            eprintln!("raea fingerprint readings: raea show didn't say its item: {said}{}", String::from_utf8_lossy(&out.stderr));
            return 1;
        };
        let mut asked: serde_json::Map<String, Value> = std::fs::read(dir.dir.join("asked.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        asked.insert(item.to_string(), json!(todo.iter().map(|&i| [r[i].note.clone(), r[i].media.display().to_string()]).collect::<Vec<_>>()));
        let _ = std::fs::write(dir.dir.join("asked.json"), serde_json::to_vec_pretty(&asked).unwrap_or_default());
        println!("{} readings put to you in the app ({item}); `raea fingerprint readings --tally` counts your answers", todo.len());
        return 0;
    }
    if has(args, "--tally") {
        let asked: serde_json::Map<String, Value> = std::fs::read(dir.dir.join("asked.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let inbox = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(format!(".raea/inbox/{INBOX}.jsonl"));
        for line in std::fs::read_to_string(inbox).unwrap_or_default().lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
            let Some(batch) = v["item"].as_str().and_then(|i| asked.get(i)).and_then(Value::as_array) else { continue };
            for (k, who) in batch.iter().enumerate() {
                let Some(a) = v["answers"][format!("q{}", k + 1)].as_str() else { continue };
                let Some((_, ok_kind, ok_axis)) = CHOICES.iter().find(|c| c.0 == a) else { continue };
                if let Some(x) = r.iter_mut().find(|x| who[0].as_str() == Some(x.note.as_str()) && who[1].as_str() == Some(&x.media.display().to_string())) {
                    (x.kind_ok, x.axis_ok) = (Some(*ok_kind), Some(*ok_axis));
                }
            }
        }
        let _ = write_readings(&dir, &r);
    }
    let (k, a, n) = agreement(&r);
    let f = |x: Option<f64>| x.map_or("-".into(), |v| format!("{:.0}%", v * 100.0));
    println!("{} readings, {n} checked: kind right {}, axis right {} (FP5b needs 50 checked, 90% and 80%): {}", r.len(), f(k), f(a), if earned(&r) { "notes may train" } else { "notes don't train yet" });
    println!("{}", json!({"readings": r.len(), "checked": n, "kind": k, "axis": a, "earned": earned(&r)}));
    0
}

/// `learn-notes`: each checked-off reading's defect or praise as an evidence row over its span, once the readings
/// have earned it; a reading already learned is replaced, so learning again is safe
pub(crate) fn learn(enc: &mut raea_sys_columns::Encoders, dir: &Folder, l: &mut raea_sys_fingerprint::evidence::Ledger) -> anyhow::Result<String> {
    let r = readings_of(dir);
    if !earned(&r) {
        let (k, a, n) = agreement(&r);
        anyhow::bail!("notes don't train yet: {n} readings checked, kind right {:?}, axis right {:?} (FP5b needs 50 checked, 90% and 80%: raea fingerprint readings --ask 50)", k, a);
    }
    // only these readings' rows are remade: notes another fingerprint's person moved up here (FP14d) stay
    let ids: std::collections::BTreeSet<&str> = r.iter().map(|x| x.note.as_str()).collect();
    l.retain(|x| x.source != "note" || !x.kind.as_deref().is_some_and(|k| ids.contains(k)));
    let mut made = 0;
    for x in r.iter().filter(|x| x.kind_ok != Some(false) && x.axis_ok != Some(false)) {
        let bad = match x.kind.as_str() {
            "defect" | "both" => true,
            "praise" => false,
            _ => continue,
        };
        let axis = match (&x.axis, bad) {
            (Some(a), _) => a.clone(),
            (None, false) => "*".into(),
            (None, true) => continue,
        };
        let c = enc.columns(&x.media)?;
        // a span is the label's time; a moment, 1.5 s before the pin to 0.5 s after (FP11b)
        let (a, b) = x.to.map_or(((x.t - 1.5).max(0.0), (x.t + 0.5).min(c.dur)), |to| (x.t.max(0.0), to.min(c.dur)));
        if b - a < 0.5 {
            continue;
        }
        // a defect's marks, grown into the thing and tracked over the label by EdgeTAM where raea-perceive is (grow.rs)
        let region = x.region.filter(|_| bad).map(|seed| raea_sys_fingerprint_use::grow::grow(&x.media, x.t, a, b, seed).ok().flatten().map_or(seed, |g| g.region));
        let row = raea_sys_fingerprint::evidence::Row {
            media: c.media.clone(),
            a,
            b,
            axis,
            bad,
            weight: 1.0,
            who: x.who.clone(),
            source: "note".into(),
            project: x.project.clone(),
            kind: Some(x.note.clone()),
            region,
            words: Some(x.text.clone()),
            at: 0,
            pin: None,
        };
        l.push(row, &raea_sys_fingerprint::evidence::features(&c, a, b));
        made += 1;
    }
    Ok(format!("{made} notes are evidence now (defects and praise; directions stay with the brief)\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_name_a_region_and_arrows_dont() {
        let pin = json!({"kind": "place", "x": 0.5, "y": 0.5, "marks": [
            {"kind": "box", "points": [[0.6, 0.7], [0.2, 0.3]]},
            {"kind": "mark", "points": [[0.9, 0.1]]},
            {"kind": "arrow", "points": [[0.0, 0.0], [1.0, 1.0]]}
        ]});
        let r = region(&pin).unwrap();
        let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(near(r[0], 0.2) && near(r[1], 0.05) && near(r[2], 0.95) && near(r[3], 0.7), "{r:?}");
        assert_eq!(region(&json!({"marks": [{"kind": "strike", "points": [[0.1, 0.1], [0.9, 0.9]]}]})), None, "a strike is for the agent");
        assert_eq!(region(&json!({"kind": "place", "x": 0.5, "y": 0.5})), None, "a pin is never a mark");
    }

    #[test]
    fn a_reading_keeps_only_kinds_and_axes_it_knows() {
        let fp = raea_sys_fingerprint::taxonomy::base("me");
        let mut n = vec![Reading { note: "n1".into(), ..Default::default() }, Reading { note: "n2".into(), ..Default::default() }, Reading { note: "n3".into(), ..Default::default() }];
        let ans = answer("I read them.\n[{\"id\": \"n1\", \"kind\": \"defect\", \"axis\": \"glitch\"}, {\"id\": \"n2\", \"kind\": \"direction\", \"axis\": \"made up\"}, {\"id\": \"n3\", \"kind\": \"rant\", \"axis\": null}]").unwrap();
        fold(&mut n, &fp, &ans, "kimi");
        assert_eq!((n[0].kind.as_str(), n[0].axis.as_deref()), ("defect", Some("glitch")));
        assert_eq!((n[1].kind.as_str(), n[1].axis.as_deref()), ("direction", None), "an axis the fingerprint hasn't is dropped");
        assert_eq!(n[2].kind, "", "a kind it doesn't know is no reading");
        assert!(prompt(&fp, &n).contains("n1: "));
    }

    #[test]
    fn notes_train_only_once_the_person_agrees_enough() {
        let r = |k: bool, a: bool| Reading { kind_ok: Some(k), axis_ok: Some(a), ..Default::default() };
        let mut v: Vec<Reading> = (0..45).map(|_| r(true, true)).collect();
        v.extend((0..5).map(|_| r(false, false)));
        assert!(earned(&v), "45 and 45 of 50");
        v[0] = r(false, true);
        assert!(!earned(&v), "44 kinds of 50 is under the bar");
        assert!(!earned(&v[..40]), "fewer than 50 checked");
    }
}
