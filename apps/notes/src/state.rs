//! What each ask has been through (docs/direction.feature DR6), kept in the project's .raea/notes-checks.json by its
//! id (n3.2: note 3's second ask): its check and values as they first failed on the work, the run before that if it
//! passed (refused: it wasn't measuring the ask), its last run, the clip cut for the person to sign and their sign.
//! From these and the note, each ask reads to check, failing, to sign, passing or signed, and the note its worst.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use raea_core_facts::{Ask, Note};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::R;

/// A run of an ask's check: when (UTC), the project's commit ("+" when not all of it was committed), what it found,
/// and what it waited for instead, when it did.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Run {
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub found: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<String>,
}

/// An ask's check as it first failed: the check, its values, and that run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Failed {
    pub check: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub values: Map<String, Value>,
    pub run: Run,
}

/// The clip cut for an ask the person signs: its file, the composition, the render it was cut from (its SHA-256),
/// the frames, and whether the composition changed after that render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub file: PathBuf,
    pub of: String,
    pub render: String,
    pub frames: [i64; 2],
    #[serde(default)]
    pub stale: bool,
}

/// The person's sign on a clip: their name (none when the app had no account to name), the render they watched, and
/// when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sign {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub render: String,
    pub at: String,
}

/// What one ask has been through.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<Failed>,
    /// its check's first run, when it passed: refused, as it wasn't measuring the ask
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<Run>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<Run>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<Clip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed: Option<Sign>,
}

/// Every ask's, by its id.
pub type State = BTreeMap<String, Kept>;

fn path(project: &Path) -> PathBuf {
    project.join(".raea/notes-checks.json")
}

pub fn read(project: &Path) -> State {
    std::fs::read_to_string(path(project)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn write(project: &Path, s: &State) -> R<()> {
    let f = path(project);
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let t = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(&f, t + "\n").map_err(|e| format!("{}: {e}", f.display()))
}

/// An ask's id: the note's number and the ask's (1 is the first).
pub fn id(note: u32, ask: usize) -> String {
    format!("n{note}.{ask}")
}

/// Where an ask stands, the word the app's badge shows, and why when that isn't plain.
#[derive(Debug, Clone, PartialEq)]
pub enum Standing {
    ToRestate,
    ToCheck(String),
    Failing(usize),
    ToSign(String),
    Passing,
    Signed(Option<String>),
}

impl Standing {
    pub fn word(&self) -> &'static str {
        match self {
            Standing::ToRestate => "to restate",
            Standing::ToCheck(_) => "to check",
            Standing::Failing(_) => "failing",
            Standing::ToSign(_) => "to sign",
            Standing::Passing => "passing",
            Standing::Signed(_) => "signed",
        }
    }

    /// The word, and why or by whom.
    pub fn said(&self) -> String {
        match self {
            Standing::ToCheck(why) | Standing::ToSign(why) => format!("{}: {why}", self.word()),
            Standing::Failing(n) => format!("failing ({n} finding{})", if *n == 1 { "" } else { "s" }),
            Standing::Signed(Some(name)) => format!("signed by {name}"),
            Standing::Signed(None) => "signed (unnamed: the app had no account)".into(),
            _ => self.word().into(),
        }
    }

    /// How far it is from settled: the note reads its asks' worst.
    fn rank(&self) -> u8 {
        match self {
            Standing::ToRestate => 5,
            Standing::ToCheck(_) => 4,
            Standing::Failing(_) => 3,
            Standing::ToSign(_) => 2,
            Standing::Passing => 1,
            Standing::Signed(_) => 0,
        }
    }
}

/// Whether the check an ask names now is the one that failed first (its check and its values).
pub fn same_check(a: &Ask, f: &Failed) -> bool {
    a.check.as_deref() == Some(f.check.as_str()) && a.values.clone().unwrap_or_default() == f.values
}

/// Where an ask stands, from the ask and what it has been through.
pub fn standing(a: &Ask, k: Option<&Kept>) -> Standing {
    let sign = a.proof.as_ref().is_some_and(|p| p.sign);
    if a.check.is_none() && !sign {
        return Standing::ToCheck("no check, and no clip to sign".into());
    }
    if a.check.is_some() {
        let failed = k.and_then(|k| k.failed.as_ref());
        match (failed, k.and_then(|k| k.last.as_ref())) {
            (Some(f), _) if !same_check(a, f) => return Standing::ToCheck("its check changed since it failed: show the new one failing (raea notes check)".into()),
            (Some(_), Some(Run { waits: Some(w), .. })) => return Standing::ToCheck(format!("its check waits for {w}")),
            (Some(_), Some(r)) if r.found > 0 => return Standing::Failing(r.found),
            (Some(_), _) => {}
            (None, _) if k.and_then(|k| k.refused.as_ref()).is_some() => return Standing::ToCheck("its check passed on its first run, so it isn't measuring the ask: write one that fails on the work as it stands".into()),
            (None, Some(Run { waits: Some(w), .. })) => return Standing::ToCheck(format!("its check waits for {w}")),
            (None, _) => return Standing::ToCheck("its check isn't shown failing on the work yet (raea notes check)".into()),
        }
        if !sign {
            return Standing::Passing;
        }
    }
    let clip = k.and_then(|k| k.clip.as_ref());
    match (clip, k.and_then(|k| k.signed.as_ref())) {
        // the agent's move until there's a clip of the work as it stands: to check, not the person's to sign
        (None, _) => Standing::ToCheck("no clip yet: render, and raea notes check cuts it from the newest render".into()),
        (Some(c), _) if c.stale => Standing::ToCheck(format!("{} changed after the render its clip is cut from: render it again", c.of)),
        (Some(c), Some(s)) if s.render == c.render => Standing::Signed(s.name.clone()),
        (Some(_), Some(s)) => Standing::ToSign(format!("{} signed an older render; its clip is new", s.name.as_deref().unwrap_or("the person"))),
        (Some(_), None) => Standing::ToSign("its clip waits on the person (raea notes post shows it)".into()),
    }
}

/// Each ask of a note with where it stands, and the note's: to restate, else its asks' worst.
pub fn of_note(n: &Note, number: u32, s: &State) -> (Standing, Vec<Standing>) {
    let asks: Vec<Standing> = n.asks.iter().enumerate().map(|(i, a)| standing(a, s.get(&id(number, i + 1)))).collect();
    if n.state() == "to restate" {
        return (Standing::ToRestate, asks);
    }
    let worst = asks.iter().max_by_key(|a| a.rank()).cloned().unwrap_or(Standing::ToRestate);
    (worst, asks)
}
