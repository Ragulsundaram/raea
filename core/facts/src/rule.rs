//! A rule: a line the work is held to, and the check that holds it (docs/direction.feature DR4). The check is one of
//! raea's, by name with its values (apps/direction systems.rs), or a project's own TS check by its path (sdk/raea
//! check.ts). Where a rule was written (raea's floor, the person's taste, a workspace's or a project's controller)
//! changes nothing about how it runs: it is only said, as its findings' `by`. A taste file's six forms are six of
//! raea's named checks with the same names and values, so every taste file reads as it did.

use std::path::Path;

use raea_core_brief::{controller, taste};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Level;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// what `--only` names: a named check's name, or a check file's name without .ts
    pub id: String,
    /// the line it holds the work to
    pub text: String,
    /// one of raea's checks by name, or a .ts check's path (resolved from the file the rule was written in)
    pub check: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub values: Map<String, Value>,
    /// how bad what it finds is (None: the check's own level, else an error)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    /// where it was written: "raea", or "taste", "workspace" or "project" and the file; "note" and the ask's id
    pub by: String,
    /// the one composition it runs on, its file from the project (its id after a `#`), as a note's ask names it;
    /// none: every composition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
}

impl Rule {
    /// One of raea's checks by `name`, held to `values`.
    pub fn named(name: &str, text: &str, values: Map<String, Value>, level: Level, by: &str) -> Rule {
        Rule { id: name.into(), text: text.into(), check: name.into(), values, level: Some(level), by: by.into(), of: None }
    }

    /// The project's .ts check this rule runs, when it runs one.
    pub fn file(&self) -> Option<&Path> {
        self.check.ends_with(".ts").then(|| Path::new(&self.check))
    }

    /// A value the rule holds its check to: a number.
    pub fn number(&self, name: &str) -> Option<f64> {
        self.values.get(name).and_then(Value::as_f64)
    }

    /// A taste line that carries a check: the named check of its form (text-hold, text-above, text-below,
    /// text-margin, loudness, still-max), its value as `value`. Taste's lines are errors, as raea check held them.
    pub fn of_taste(text: &str, c: taste::Check, file: &str) -> Rule {
        let values = Map::from_iter([("value".to_string(), Value::from(c.value()))]);
        Rule::named(c.form(), text, values, Level::Error, &format!("taste {file}"))
    }

    /// A controller's rule fields, each a rule: a check's path joined to `dir` (the controller's folder; its `..`
    /// steps are the caller's to take), the field's label its words. `by`: "workspace" or "project", with the
    /// controller's file.
    pub fn of_controller(c: &controller::Controller, dir: &Path, by: &str) -> Vec<Rule> {
        let mut out = Vec::new();
        for f in c.groups.iter().flat_map(|g| &g.fields) {
            let controller::Kind::Rule { check, values, level } = &f.kind else { continue };
            let (check, id) = match check.ends_with(".ts") {
                true => {
                    let path = dir.join(check);
                    let id = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    (path.display().to_string(), id)
                }
                false => (check.clone(), check.clone()),
            };
            let level = level.as_deref().map(Level::of_word);
            out.push(Rule { id, text: f.label.clone(), check, values: values.clone().unwrap_or_default(), level, by: by.into(), of: None });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taste_form_is_the_named_check_of_the_same_name_and_value() {
        let r = Rule::of_taste("Captions sit clear of the bottom.", taste::Check::TextAbove(62.0), "taste.ts");
        assert_eq!((r.id.as_str(), r.check.as_str(), r.number("value"), r.level), ("text-above", "text-above", Some(62.0), Some(Level::Error)));
        assert!(r.file().is_none() && r.by == "taste taste.ts");
    }

    #[test]
    fn a_controllers_rule_fields_are_rules_and_a_path_is_from_its_folder() {
        let raw = serde_json::json!({"title": "Rules", "groups": [{"title": "G", "fields": [
            {"label": "Captions clear the button band", "type": "rule", "check": "../checks/clear.ts", "level": "warn"},
            {"label": "Loud enough", "type": "rule", "check": "loudness", "values": {"value": -14}},
            {"label": "Burn in captions", "type": "toggle", "value": true}]}]});
        let c = controller::normalize(&raw).unwrap();
        let rs = Rule::of_controller(&c, Path::new("/p/controllers"), "project controllers/rules.ts");
        assert_eq!(rs.len(), 2, "a toggle is no rule");
        assert_eq!((rs[0].id.as_str(), rs[0].file(), rs[0].level), ("clear", Some(Path::new("/p/controllers/../checks/clear.ts")), Some(Level::Warn)));
        assert_eq!((rs[1].check.as_str(), rs[1].number("value"), rs[1].level), ("loudness", Some(-14.0), None));
    }
}
