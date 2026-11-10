//! A person's note, and the asks it holds once restated (docs/direction.feature DR5). A note is a file in the project,
//! notes/<round>/<id>.ts, whose default export is `note({...})` (raea/note): the person's words verbatim, where they
//! came from, where on the work they were left, and the asks the agent restates them as, one for each thing they ask
//! for. Nothing in the work changes before a note's asks are written. In a world, a note is an entity with a Note.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

/// Where on the work a note was left: a moment (seconds), a place on the frame (0 to 1 across and down), and the
/// part there, as the app's pin has them.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Pin {
    pub t: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub part: Option<String>,
}

/// Where an ask that can't be measured is shown, for the person to sign (DR6): a shot or a range of frames (the frame
/// after its last ends it) of the ask's composition, cut from its newest render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Proof {
    pub shot: Option<String>,
    pub frames: Option<[i64; 2]>,
    pub sign: bool,
}

/// One thing a note asks for, in the agent's words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Ask {
    /// the one thing asked
    pub text: String,
    /// the medium the person's words name for it, in their word: "2D", "3D", "flat", "extruded", "real",
    /// "modelled", "voxel"
    pub medium: Option<String>,
    /// what it is about: the part the pin is on (found in the world), or what a check would query
    pub about: Option<String>,
    /// what the note does not ask for, so a change keeps it (for a redesign: what is kept)
    pub not: Vec<String>,
    /// the craft verb this ask is (skills/skills/raea/craft/verbs): one per ask, not one per note
    pub verb: Option<String>,
    /// the composition it is about, its file from the project (its id after a `#` where the file registers several):
    /// where its check runs and its clip is cut from; none: every composition of the project
    pub of: Option<String>,
    /// the check that shows it met (DR6): one of raea's by name, or a .ts check by its path, as a rule names one
    pub check: Option<String>,
    /// the values its check is held to
    #[ts(type = "Record<string, unknown>", optional)]
    pub values: Option<Map<String, Value>>,
    /// where it can't be measured: the clip the person signs
    pub proof: Option<Proof>,
}

/// A person's note: their words, verbatim (the first line as sent; each later line a correction they made, kept),
/// where it came from ("app ITEM COMMENT" for a comment sent from the app, "session" for one said in a session), the
/// round it came in, its pin, whether it asks for a redesign, the asks the person corrected since it was restated (1
/// is the first, 0 the note as a whole: each restated again, then taken off), and its asks (none yet: to restate).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Note {
    pub words: Vec<String>,
    pub from: String,
    pub round: u32,
    pub pin: Option<Pin>,
    pub redesign: Option<bool>,
    pub corrected: Option<Vec<u32>>,
    pub asks: Vec<Ask>,
}

crate::components::component!(Note);

/// Words that ask for a thing to be made again rather than changed.
const REDESIGN: [&str; 7] = ["from scratch", "redesign", "not the vibe", "start over", "rethink", "redo", "all over again"];

/// The media a person's words can name, each as said and the forms it is said in.
const MEDIA: [(&str, &[&str]); 7] = [
    ("2D", &["2d"]),
    ("3D", &["3d"]),
    ("flat", &["flat"]),
    ("extruded", &["extruded", "extrusion", "extrude"]),
    ("real", &["real"]),
    ("modelled", &["modelled", "modeled", "modelling", "modeling"]),
    ("voxel", &["voxel", "voxels"]),
];

/// How many sentences `words` holds: ends of sentence (. ! ? ;) and line breaks, between which there is something.
pub fn sentences(words: &str) -> usize {
    words.split(['.', '!', '?', ';', '\n']).filter(|s| s.chars().any(char::is_alphanumeric)).count()
}

/// Whether `words` ask for a redesign.
pub fn redesign(words: &str) -> bool {
    let w = words.to_lowercase();
    REDESIGN.iter().any(|r| w.contains(r))
}

/// The media `words` name, each as a person would write it ("2D").
pub fn mediums(words: &str) -> Vec<&'static str> {
    let said: Vec<String> = words.split(|c: char| !c.is_alphanumeric()).map(str::to_lowercase).collect();
    MEDIA.iter().filter(|(_, forms)| said.iter().any(|w| forms.contains(&w.as_str()))).map(|(m, _)| *m).collect()
}

impl Note {
    /// Where it stands: "to restate" (no asks yet, or the person corrected one), "to check" (an ask has no check and
    /// no clip to sign: DR6), or "checked".
    pub fn state(&self) -> &'static str {
        if self.asks.is_empty() || self.corrected.as_ref().is_some_and(|c| !c.is_empty()) {
            "to restate"
        } else if self.asks.iter().any(|a| a.check.is_none() && a.proof.is_none()) {
            "to check"
        } else {
            "checked"
        }
    }

    /// What its restatement still owes, once it has asks: a second reading where many sentences gave one ask, what a
    /// redesign keeps, and each medium its words name that no ask asks for or turns down.
    pub fn flags(&self) -> Vec<String> {
        if self.asks.is_empty() {
            return Vec::new();
        }
        // all its words, the correction lines after the first
        let said = self.words.join("\n");
        let mut out: Vec<String> = self.corrected.iter().flatten().map(|n| match n {
            0 => "the person corrected the note (their line is the last of its words): restate it again, then take 0 off `corrected`".to_string(),
            n => format!("the person corrected ask {n} (their line is the last of its words): restate it again, then take {n} off `corrected`"),
        }).collect();
        // the note as first sent: a correction refines the asks, it isn't more of them
        let n = sentences(self.words.first().map_or("", String::as_str));
        if self.asks.len() == 1 && n > 1 {
            out.push(format!("one ask from {n} sentences: read it again for what else it asks"));
        }
        if self.redesign == Some(true) && self.asks.iter().all(|a| a.not.is_empty()) {
            out.push("it asks for a redesign: say what is kept (an ask's `not`)".into());
        }
        // a medium the words name is asked for (an ask's medium) or turned down (in an ask's `not`)
        let given: Vec<&str> = self.asks.iter().flat_map(|a| a.medium.iter().chain(&a.not)).flat_map(|s| mediums(s)).collect();
        for m in mediums(&said).into_iter().filter(|m| !given.contains(m)) {
            out.push(format!("its words say {m} and no ask says it, as its medium or in its `not`"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(text: &str, medium: Option<&str>, not: &[&str]) -> Ask {
        Ask { text: text.into(), medium: medium.map(Into::into), about: None, not: not.iter().map(|s| s.to_string()).collect(), verb: None, of: None, check: None, values: None, proof: None }
    }

    fn note(words: &str, asks: Vec<Ask>) -> Note {
        Note { words: vec![words.into()], from: "session".into(), round: 1, pin: None, redesign: redesign(words).then_some(true), corrected: None, asks }
    }

    #[test]
    fn a_corrected_ask_is_restated_again() {
        let mut n = note("make the title 2D", vec![ask("the title is flat type", Some("2D"), &[])]);
        n.words.push("on ask 1: and it sits behind the singer".into());
        n.corrected = Some(vec![1]);
        assert_eq!(n.state(), "to restate");
        assert_eq!(n.flags(), ["the person corrected ask 1 (their line is the last of its words): restate it again, then take 1 off `corrected`"]);
        n.corrected = Some(vec![]);
        assert_eq!((n.state(), n.flags().len()), ("to check", 0));
    }

    #[test]
    fn a_note_is_to_restate_until_it_has_asks_then_to_check_until_each_has_a_check() {
        let mut n = note("make the title 2D", vec![]);
        assert_eq!((n.state(), n.flags().len()), ("to restate", 0));
        n.asks = vec![ask("the title is flat type", Some("2D"), &[])];
        assert_eq!(n.state(), "to check");
        n.asks[0].check = Some("text-flat".into());
        assert_eq!(n.state(), "checked");
    }

    #[test]
    fn many_sentences_restated_as_one_ask_are_read_again() {
        let n = note("The desks face the wall. And why are there kitchen props on them? Make it an office.", vec![ask("turn the desks", None, &[])]);
        assert_eq!(n.flags(), ["one ask from 3 sentences: read it again for what else it asks"]);
        let n = note("turn the desks around", vec![ask("turn the desks", None, &[])]);
        assert!(n.flags().is_empty());
    }

    #[test]
    fn the_medium_the_words_name_is_an_asks() {
        let n = note("i want 2D animated text with occlusion", vec![ask("lyrics behind the singer", None, &[])]);
        assert_eq!(n.flags(), ["its words say 2D and no ask says it, as its medium or in its `not`"]);
        let n = note("i want 2D animated text with occlusion", vec![ask("lyrics behind the singer", Some("2D"), &[])]);
        assert!(n.flags().is_empty());
        // what the person turns down is said too: in an ask's `not`
        let n = note("2D animated text, not 3D text extrusion", vec![ask("lyrics behind the singer", Some("2D"), &["3D type"])]);
        assert_eq!(n.flags(), ["its words say extruded and no ask says it, as its medium or in its `not`"]);
        let n = note("2D animated text, not 3D text extrusion", vec![ask("lyrics behind the singer", Some("2D"), &["3D type", "extruded letters"])]);
        assert!(n.flags().is_empty());
        assert_eq!(mediums("real modeled places, not voxel or 3d text extrusion"), ["3D", "extruded", "real", "modelled", "voxel"]);
        assert!(mediums("really flatter").is_empty(), "a word that only starts like a medium isn't one");
    }

    #[test]
    fn a_redesign_says_what_it_keeps() {
        let n = note("this isn't the vibe, redo it from scratch", vec![ask("a new look for the room", None, &[])]);
        assert_eq!(n.redesign, Some(true));
        assert_eq!(n.flags(), ["it asks for a redesign: say what is kept (an ask's `not`)"]);
        let n = note("this isn't the vibe, redo it from scratch", vec![ask("a new look for the room", None, &["the camera moves", "the cut"])]);
        assert!(n.flags().is_empty());
        assert!(!redesign("move the lamp left"));
    }
}
