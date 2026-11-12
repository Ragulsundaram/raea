//! A workspace's controllers on disk (docs/projects.feature PJ2): shared/controllers/<id>.ts (docs/layout.feature; a
//! workspace from before shared/ keeps controllers/ at its root), each `export default
//! controller({...})` from "raea/brief", checked by core/brief's normalize. A project overrides a field's value in its
//! .raea/project (`[controller "delivery.format"] aspect-ratio = 16:9`), so the project's value wins, as git config's
//! own scopes do. Every change is a commit by who made it ("you", or the agent's name with --by).
use super::*;
use raea_core_brief::controller::{self as ctl, Controller, Field, Kind};

use std::process::Command;

const USAGE: &str = "usage: raea controller ls [--json] | get PATH [--json] | set PATH VALUE [--project] [--by NAME] | write FILE.ts [--by NAME] | rm ID [--by NAME]\n       (PATH is controller.group.field; a VALUE is JSON for numbers, toggles and lists, else text)";

fn workspace_dir() -> R<PathBuf> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    workspace::up(&here, ".raea/workspace").ok_or_else(|| "not in a workspace: raea workspace init NAME makes one".to_string())
}

/// Every controller in the workspace, checked, in file order.
pub fn load(ws: &Path, eval: Eval) -> R<Vec<(PathBuf, Controller)>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(raea_app_registry::shared_dir(ws, "controllers")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ts")).collect();
    files.sort();
    files.into_iter().map(|f| read(&f, eval).map(|c| (f, c))).collect()
}

fn read(f: &Path, eval: Eval) -> R<Controller> {
    ctl::of_json(&eval(f)?).map_err(|e| format!("{}: {e}", f.display()))
}

/// The module a controller is kept as.
pub fn module(c: &Controller) -> String {
    format!("import {{ controller }} from \"raea/brief\";\n\nexport default controller({});\n", ts_literal(&serde_json::to_value(c).unwrap_or_default(), 0))
}

/// A field's value as JSON (its kind's `value`).
fn value_of(f: &Field) -> serde_json::Value {
    serde_json::to_value(&f.kind).ok().and_then(|v| v.get("value").cloned()).unwrap_or_default()
}

/// The git-config key for a field's override: `controller.<controller>.<group>.<field>`, which git reads as section
/// controller, subsection "<controller>.<group>", key <field>.
fn key(path: &str) -> String {
    format!("controller.{path}")
}

/// A value as typed on the command line or kept in git config: JSON when it parses as JSON of the field's kind, else
/// the text itself (a choice, a text field).
fn parse(f: &Field, s: &str) -> R<serde_json::Value> {
    let v = match (&f.kind, serde_json::from_str::<serde_json::Value>(s)) {
        (Kind::Text { .. } | Kind::Choice { .. }, Ok(serde_json::Value::String(t))) => serde_json::Value::String(t),
        (Kind::Text { .. } | Kind::Choice { .. }, _) => serde_json::Value::String(s.to_string()),
        (_, Ok(v)) => v,
        (_, Err(_)) => serde_json::Value::String(s.to_string()),
    };
    ctl::check_value(f, &v)
}

/// The overrides of the project `from` is in, by path.
pub(crate) fn overrides(from: &Path) -> Vec<(String, String)> {
    let Some(p) = workspace::up(from, ".raea/project") else { return vec![] };
    let text = workspace::config(Some(&p.join(".raea/project")), &["--get-regexp", "^controller\\."]).unwrap_or_default();
    text.lines().filter_map(|l| l.split_once(' ')).map(|(k, v)| (k.trim_start_matches("controller.").to_string(), v.to_string())).collect()
}

fn commit(ws: &Path, paths: &[&Path], what: &str, by: &str) -> R<()> {
    let mut add = Command::new("git");
    add.arg("-C").arg(ws).args(["add", "-A", "--"]).args(paths);
    add.status().map_err(|e| format!("git: {e}"))?;
    let ok = workspace::git_cmd().arg("-C").arg(ws).args(["-c", &format!("user.name={by}"), "commit", "-q", "-m", &format!("{by}: {what}"), "--"]).args(paths).status().map_err(|e| format!("git: {e}"))?;
    if ok.success() { Ok(()) } else { Err(format!("couldn't commit {what}")) }
}

/// `raea controller …`, its specs evaluated by `eval`
pub fn controller(args: &[String], eval: Eval) -> R<String> {
    let json_out = args.iter().any(|a| a == "--json");
    let by = workspace::flag(args, "--by").unwrap_or("you").trim_start_matches('@').to_string();
    let w: Vec<&str> = args.iter().enumerate().filter(|(i, a)| !a.starts_with("--") && !(*i > 0 && args[i - 1] == "--by")).map(|(_, a)| a.as_str()).collect();
    let ws = workspace_dir()?;
    match w.as_slice() {
        ["ls"] | [] => ls(&ws, json_out, eval),
        ["get", path] => get(&ws, path, json_out, eval),
        ["set", path, value] => set(&ws, path, value, args.iter().any(|a| a == "--project"), &by, eval),
        ["write", file] => write(&ws, Path::new(file), &by, eval),
        ["rm", id] => rm(&ws, id, &by),
        _ => Err(USAGE.into()),
    }
}

/// The controllers as `raea controller ls` prints them, with the overrides of the project `from` is in: what `raea
/// context` shows an agent. None outside a workspace or when it has no controllers.
pub fn summary(from: &Path, eval: Eval) -> Option<String> {
    let ws = workspace::up(from, ".raea/workspace")?;
    load(&ws, eval).ok().filter(|cs| !cs.is_empty())?;
    ls_from(&ws, from, false, eval).ok()
}

fn ls(ws: &Path, json_out: bool, eval: Eval) -> R<String> {
    ls_from(ws, &std::env::current_dir().map_err(|e| e.to_string())?, json_out, eval)
}

fn ls_from(ws: &Path, from: &Path, json_out: bool, eval: Eval) -> R<String> {
    let cs = load(ws, eval)?;
    let over = overrides(from);
    let mut rows = vec![];
    for (_, c) in &cs {
        for g in &c.groups {
            for f in &g.fields {
                let path = format!("{}.{}.{}", c.id, g.id, f.id);
                let mine = over.iter().find(|(k, _)| *k == path).and_then(|(_, v)| parse(f, v).ok());
                rows.push((path, f, mine.clone().unwrap_or_else(|| value_of(f)), mine.is_some()));
            }
        }
    }
    if json_out {
        let fields: Vec<_> = rows.iter().map(|(p, f, v, o)| json!({"path": p, "label": f.label, "type": serde_json::to_value(&f.kind).ok().and_then(|k| k.get("type").cloned()), "value": v, "project": o})).collect();
        let list: Vec<_> = cs.iter().map(|(_, c)| serde_json::to_value(c).map(|v| tidy_deep(&v)).unwrap_or_default()).collect();
        return Ok(json!({"controllers": list, "fields": fields}).to_string());
    }
    if cs.is_empty() {
        return Ok("no controllers yet: raea controller write FILE.ts adds one (`export default controller({ title, groups })` from \"raea/brief\")".into());
    }
    let mut out = String::new();
    for (_, c) in &cs {
        out.push_str(&format!("{} ({})\n", c.title, c.id));
        for (p, f, v, o) in rows.iter().filter(|r| r.0.starts_with(&format!("{}.", c.id))) {
            out.push_str(&format!("  {p}  {}: {}{}\n", f.label, tidy(v), if *o { "  (this project)" } else { "" }));
        }
    }
    Ok(out.trim_end().to_string())
}

/// Where a path's field is: its controller, group and field, by index.
fn locate(cs: &[(PathBuf, Controller)], path: &str) -> R<(usize, usize, usize)> {
    let mut p = path.splitn(3, '.');
    let (c, g, f) = (p.next().unwrap_or(""), p.next().unwrap_or(""), p.next().unwrap_or(""));
    let ci = cs.iter().position(|x| x.1.id == c);
    let gi = ci.and_then(|ci| cs[ci].1.groups.iter().position(|y| y.id == g));
    let fi = ci.zip(gi).and_then(|(ci, gi)| cs[ci].1.groups[gi].fields.iter().position(|z| z.id == f));
    match (ci, gi, fi) {
        (Some(c), Some(g), Some(f)) => Ok((c, g, f)),
        _ => Err(format!("no field at {path}: raea controller ls lists the paths (controller.group.field)")),
    }
}

fn find<'a>(cs: &'a [(PathBuf, Controller)], path: &str) -> R<&'a Field> {
    let (c, g, f) = locate(cs, path)?;
    Ok(&cs[c].1.groups[g].fields[f])
}

fn get(ws: &Path, path: &str, json_out: bool, eval: Eval) -> R<String> {
    let cs = load(ws, eval)?;
    let f = find(&cs, path)?;
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let mine = overrides(&here).into_iter().find(|(k, _)| k == path).map(|(_, v)| parse(f, &v)).transpose()?;
    let v = mine.clone().unwrap_or_else(|| value_of(f));
    let v = tidy(&v);
    Ok(if json_out { json!({"path": path, "value": v, "project": mine.is_some()}).to_string() } else { v.to_string() })
}

fn set(ws: &Path, path: &str, value: &str, project: bool, by: &str, eval: Eval) -> R<String> {
    let mut cs = load(ws, eval)?;
    let v = parse(find(&cs, path)?, value)?;
    if project {
        let here = std::env::current_dir().map_err(|e| e.to_string())?;
        let p = workspace::up(&here, ".raea/project").ok_or("--project: not in a project")?;
        let text = match &v {
            serde_json::Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let file = p.join(".raea/project");
        workspace::set(&file, &key(path), &text)?;
        // a submodule project is its own repo; an inline one commits in the workspace
        let repo = if p.join(".git").exists() { p.clone() } else { ws.to_path_buf() };
        commit(&repo, &[&file], &format!("{path} = {text} for this project"), by)?;
        return Ok(format!("{path} = {text} (this project)"));
    }
    let (c, g, f) = locate(&cs, path)?;
    let (file, ctrl) = &mut cs[c];
    let field = &mut ctrl.groups[g].fields[f];
    let mut kind = serde_json::to_value(&field.kind).map_err(|e| e.to_string())?;
    kind["value"] = v.clone();
    field.kind = serde_json::from_value(kind).map_err(|e| e.to_string())?;
    std::fs::write(&*file, module(ctrl)).map_err(|e| format!("{}: {e}", file.display()))?;
    let file = file.clone();
    let v = tidy(&v);
    let said = v.as_str().map(String::from).unwrap_or_else(|| v.to_string());
    commit(ws, &[&file], &format!("{path} = {said}"), by)?;
    Ok(format!("{path} = {v}"))
}

fn write(ws: &Path, file: &Path, by: &str, eval: Eval) -> R<String> {
    let c = read(file, eval)?;
    let to = raea_app_registry::shared_dir(ws, "controllers").join(format!("{}.ts", c.id));
    let new = !to.exists();
    std::fs::create_dir_all(raea_app_registry::shared_dir(ws, "controllers")).map_err(|e| e.to_string())?;
    std::fs::write(&to, module(&c)).map_err(|e| format!("{}: {e}", to.display()))?;
    commit(ws, &[&to], &format!("{} controller {}", if new { "add" } else { "rewrite" }, c.id), by)?;
    let n = |k: usize, what: &str| format!("{k} {what}{}", if k == 1 { "" } else { "s" });
    Ok(format!("{}: {} ({}, {})", to.display(), c.title, n(c.groups.len(), "group"), n(c.groups.iter().map(|g| g.fields.len()).sum(), "field")))
}

fn rm(ws: &Path, id: &str, by: &str) -> R<String> {
    let file = raea_app_registry::shared_dir(ws, "controllers").join(format!("{id}.ts"));
    if !file.is_file() {
        return Err(format!("no controller {id}: raea controller ls lists them"));
    }
    std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    commit(ws, &[&file], &format!("remove controller {id}"), by)?;
    Ok(format!("removed {id} (git revert brings it back)"))
}
