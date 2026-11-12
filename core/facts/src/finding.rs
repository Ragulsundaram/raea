//! One finding, for every command (docs/direction.feature DR3): what the clipping check, `raea scene lint`, `raea check`
//! and the rest write, in one shape, so "everything found about this part" is one query and a count means the same
//! thing whichever command printed it. raea check's ranks and lint's words map onto its three levels.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How bad: an error stops a render's exit (unless `--despite`), a warn is shown, a note is said. Ordered worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warn,
    Note,
}

impl Level {
    /// raea check's ranks: P0 and P1 are errors, P2 a warn, P3 (and anything past it) a note.
    pub fn of_rank(p: u8) -> Level {
        match p {
            0 | 1 => Level::Error,
            2 => Level::Warn,
            _ => Level::Note,
        }
    }

    /// lint's and the older commands' words: "error", "warning" or "warn"; anything else is a note.
    pub fn of_word(w: &str) -> Level {
        match w {
            "error" => Level::Error,
            "warning" | "warn" => Level::Warn,
            _ => Level::Note,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Note => "note",
        }
    }
}

/// An entity a finding is about, by what names it across renders: its layer key (a Source's key, "file:line:col…")
/// when it has one, and the name it goes by (a part's label, a text's words).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct About {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(optional)]
    pub key: Option<String>,
}

/// The number a finding turns on: what was measured, its value and unit, and the limit it was held to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Measure {
    pub name: String,
    pub value: f64,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(optional)]
    pub limit: Option<f64>,
}

/// One thing found: by which check, how bad, about which entities, on which frames, in a sentence, with the number it
/// turns on and how to fix it, and what found it (a command, or the ask a note's check was written for, DR6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Finding {
    pub check: String,
    pub level: Level,
    #[serde(default)]
    pub about: Vec<About>,
    /// from its first frame to the frame after its last; None when it is about the whole piece
    #[serde(default)]
    pub frames: Option<[i64; 2]>,
    pub says: String,
    #[serde(default)]
    pub measure: Option<Measure>,
    #[serde(default)]
    pub fix: Option<String>,
    pub by: String,
}

impl Finding {
    pub fn new(check: &str, level: Level, says: impl Into<String>, by: &str) -> Finding {
        Finding { check: check.into(), level, about: Vec::new(), frames: None, says: says.into(), measure: None, fix: None, by: by.into() }
    }

    pub fn about(mut self, name: impl Into<String>, key: Option<String>) -> Finding {
        self.about.push(About { name: name.into(), key });
        self
    }

    pub fn frames(mut self, from: i64, to: i64) -> Finding {
        self.frames = Some([from, to]);
        self
    }

    pub fn measure(mut self, name: &str, value: f64, unit: &str, limit: Option<f64>) -> Finding {
        self.measure = Some(Measure { name: name.into(), value, unit: unit.into(), limit });
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Finding {
        self.fix = Some(fix.into());
        self
    }

    /// One kept as JSON (in a report or the record), read back.
    pub fn read(v: &serde_json::Value) -> Option<Finding> {
        Finding::deserialize(v).ok()
    }

    /// A finding in the JSON the older commands write (`{check, level, says}` and what it is about: `part`, `into`,
    /// `on`, `file`; scene lint, gen check, solid info): None for "ok", a pass. A `fix` that is an object says its
    /// command (`run`) or else itself as JSON.
    pub fn of_value(v: &serde_json::Value, by: &str) -> Option<Finding> {
        let word = v["level"].as_str().unwrap_or("");
        if word == "ok" {
            return None;
        }
        let mut f = Finding::new(v["check"].as_str().unwrap_or(by), Level::of_word(word), v["says"].as_str().unwrap_or_default(), by);
        for k in ["part", "into", "on", "file"] {
            if let Some(name) = v[k].as_str() {
                f = f.about(name, None);
            }
        }
        f.fix = match &v["fix"] {
            serde_json::Value::String(s) => Some(s.clone()),
            o if o.is_object() => Some(o["run"].as_str().map_or_else(|| o.to_string(), str::to_string)),
            _ => None,
        };
        Some(f)
    }

    /// One line: `error clips  frames 120-139  seat, rider: the rider is 4 cm into the seat (fix: …)`.
    pub fn line(&self) -> String {
        let mut s = format!("{:<5} {}", self.level.word(), self.check);
        if let Some([a, b]) = self.frames {
            s.push_str(&if b <= a + 1 { format!("  frame {a}") } else { format!("  frames {a}-{}", b - 1) });
        }
        if !self.about.is_empty() {
            s.push_str("  ");
            s.push_str(&self.about.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "));
        }
        s.push_str(": ");
        s.push_str(&self.says);
        if let Some(f) = &self.fix {
            s.push_str(&format!(" (fix: {f})"));
        }
        s
    }
}

static SAID: std::sync::Mutex<Vec<Finding>> = std::sync::Mutex::new(Vec::new());
/// Held by a test that says and takes, so two such tests don't take each other's findings
#[cfg(test)]
pub(crate) static SAYING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Findings a command has said as it went (a render's clipping, from its own thread), kept for its count at the end.
pub fn said(fs: &[Finding]) {
    SAID.lock().unwrap_or_else(|e| e.into_inner()).extend(fs.iter().cloned());
}

/// A finding about frame `f` alone, said: printed and kept, unless the same finding is running on the frames just before
/// it, which it then joins (a render's black frames with paint on them say one line a run, and count once: #96).
pub fn said_frame(found: Finding, f: i64) {
    let mut all = SAID.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(run) = all.iter_mut().rev().find(|x| x.check == found.check && x.says == found.says && x.frames.is_some_and(|[_, b]| b == f)) {
        run.frames = run.frames.map(|[a, _]| [a, f + 1]);
        return;
    }
    let found = found.frames(f, f + 1);
    eprintln!("raea: {}", found.line());
    all.push(found);
}

/// Every finding said so far, taken: what the command ends by counting.
pub fn taken() -> Vec<Finding> {
    std::mem::take(&mut *SAID.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The findings said so far, written to `to` (beside a video in the export cache, which is copied later instead of
/// drawn and checked again): whether they were.
pub fn keep_to(to: &std::path::Path) -> bool {
    serde_json::to_vec(&*SAID.lock().unwrap_or_else(|e| e.into_inner())).is_ok_and(|b| std::fs::write(to, b).is_ok())
}

/// Findings kept by keep_to, said again as when they were found: printed, and counted at the end.
pub fn replay(from: &std::path::Path) {
    let fs: Vec<Finding> = std::fs::read(from).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    for f in &fs {
        eprintln!("raea: {}", f.line());
    }
    said(&fs);
}

/// Findings worst first, each level in the order found.
pub fn ranked(mut fs: Vec<Finding>) -> Vec<Finding> {
    fs.sort_by_key(|f| f.level);
    fs
}

/// The count a command prints last: "2 errors, 1 warn, 3 notes".
pub fn count(fs: &[Finding]) -> String {
    let say = |l: Level| match fs.iter().filter(|f| f.level == l).count() {
        1 => format!("1 {}", l.word()),
        k => format!("{k} {}s", l.word()),
    };
    [Level::Error, Level::Warn, Level::Note].map(say).join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finding_on_frames_in_a_row_is_one_finding_over_them() {
        let _held = SAYING.lock().unwrap_or_else(|e| e.into_inner());
        let _ = taken();
        let lost = || Finding::new("black-frame", Level::Warn, "drew all black", "raea render");
        for f in [22, 23, 24, 30] {
            said_frame(lost(), f);
        }
        said_frame(Finding::new("black-frame", Level::Warn, "other", "raea render"), 25);
        let got: Vec<_> = taken().into_iter().map(|x| (x.says, x.frames)).collect();
        assert_eq!(got, vec![("drew all black".to_string(), Some([22, 25])), ("drew all black".into(), Some([30, 31])), ("other".into(), Some([25, 26]))]);
    }

    #[test]
    fn the_old_ranks_and_words_map_onto_three_levels() {
        assert_eq!([0, 1, 2, 3].map(Level::of_rank), [Level::Error, Level::Error, Level::Warn, Level::Note]);
        assert_eq!(["error", "warning", "warn", "info"].map(Level::of_word), [Level::Error, Level::Warn, Level::Warn, Level::Note]);
    }

    #[test]
    fn a_finding_reads_as_one_line_and_as_json_and_back() {
        let f = Finding::new("clips", Level::Error, "the rider is 4 cm into the seat", "raea render")
            .about("rider", Some("scene.tsx:12:5".into()))
            .about("seat", None)
            .frames(120, 140)
            .measure("depth", 0.04, "m", Some(0.005))
            .fix("raise the rider 4 cm");
        assert_eq!(f.line(), "error clips  frames 120-139  rider, seat: the rider is 4 cm into the seat (fix: raise the rider 4 cm)");
        let j = serde_json::to_value(&f).unwrap();
        assert_eq!((j["level"].as_str(), j["about"][1].get("key")), (Some("error"), None));
        assert_eq!(serde_json::from_value::<Finding>(j).unwrap(), f);
    }

    #[test]
    fn the_older_commands_json_reads_as_findings_and_a_pass_as_none() {
        let v = serde_json::json!({"check": "format", "level": "error", "file": "chair.fbx", "says": "raea loads glTF", "fix": {"run": "raea blender run convert.py"}});
        let f = Finding::of_value(&v, "raea gen check").unwrap();
        assert_eq!((f.level, f.about[0].name.as_str(), f.fix.as_deref()), (Level::Error, "chair.fbx", Some("raea blender run convert.py")));
        assert_eq!(Finding::of_value(&serde_json::json!({"check": "expect.volume", "level": "warning", "says": "s", "fix": {"scale": 2}}), "x").unwrap().fix.as_deref(), Some("{\"scale\":2}"));
        assert_eq!(Finding::of_value(&serde_json::json!({"check": "expect.volume", "level": "ok", "says": "within"}), "x"), None);
    }

    #[test]
    fn findings_rank_worst_first_and_count() {
        let fs = vec![Finding::new("a", Level::Note, "n", "x"), Finding::new("b", Level::Error, "e", "x"), Finding::new("c", Level::Warn, "w", "x")];
        let fs = ranked(fs);
        assert_eq!(fs.iter().map(|f| f.check.as_str()).collect::<Vec<_>>(), ["b", "c", "a"]);
        assert_eq!(count(&fs), "1 error, 1 warn, 1 note");
    }
}
