//! Workspaces and projects on disk (docs/projects.feature PJ1). A workspace is a git repo holding .raea/workspace, a
//! git-config file that is committed, and projects/<name>/, each holding .raea/project. raea writes an include into
//! the repo's own .git/config, so `git config workspace.layout` answers from the committed file; git never loads a
//! committed config by itself. Every change raea makes here is a commit.
use super::*;
use std::process::Command;

const USAGE: &str = "usage: raea workspace [--json] | raea workspace init [NAME] [--layout monorepo|submodules] | raea workspace adopt DIR [--name NAME] [--layout monorepo|submodules]\n       raea project [--json] | raea project new NAME [--storage inline|submodule] | raea project split NAME [--remote URL] | raea project join NAME | raea project scan [DIR] [--json]";

/// git for raea's own commits. A new Mac has no user.email and a hostname git can't make one from, so every commit
/// failed with "Author identity unknown" and the person's first project was never made. git reads $EMAIL only when
/// user.email isn't set, so this gives a fresh Mac an address without overriding anyone's own. A Linux account with no
/// full name (a CI runner's, a container's) has no name either ("empty ident name ... not allowed"), and git has no
/// fallback below user.name to give it, so when no user.name is set the author and committer are named for the account.
pub(crate) fn git_cmd() -> Command {
    let mut c = Command::new("git");
    let user = std::env::var("USER").unwrap_or_else(|_| "raea".into());
    if std::env::var_os("EMAIL").is_none() {
        c.env("EMAIL", format!("{user}@raea.local"));
    }
    if !has_name() {
        c.env("GIT_AUTHOR_NAME", &user).env("GIT_COMMITTER_NAME", &user);
    }
    c
}

/// Whether git has a user.name to sign commits with (asked once a process).
fn has_name() -> bool {
    static HAS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HAS.get_or_init(|| Command::new("git").args(["config", "--get", "user.name"]).output().is_ok_and(|o| o.status.success() && !o.stdout.trim_ascii().is_empty()))
}

pub(crate) fn git(dir: &Path, args: &[&str]) -> R<String> {
    let out = git_cmd().arg("-C").arg(dir).args(args).output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `git config` on a file (or the global config when `file` is None), its output trimmed; Err when git says no.
pub(crate) fn config(file: Option<&Path>, args: &[&str]) -> R<String> {
    let mut c = Command::new("git");
    c.arg("config");
    if let Some(f) = file {
        c.arg("-f").arg(f);
    }
    let out = c.args(args).output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git config {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A git-config file's value, None when unset.
pub(crate) fn get(file: &Path, key: &str) -> Option<String> {
    config(Some(file), &[key]).ok().filter(|v| !v.is_empty())
}

pub(crate) fn set(file: &Path, key: &str, value: &str) -> R<()> {
    let dir = file.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    config(Some(file), &[key, value]).map(drop)
}

/// Every key of a git-config file, in order, as (key, value).
fn all(file: &Path) -> Vec<(String, String)> {
    let list = config(Some(file), &["--list", "-z"]).unwrap_or_default();
    list.split('\0').filter_map(|e| e.split_once('\n')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// The value after `name` in args.
pub(crate) fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// The words of a command that aren't flags or a flag's value.
fn words(args: &[String]) -> Vec<&str> {
    raea_app_check::words(args, &["--layout", "--storage", "--name", "--remote"])
}

// a name as a folder (slug), and the nearest folder holding a marker (up): the registry's, which reads controller ids
// and projects with them
pub use raea_app_registry::{slug, up};

/// A new id: the prefix and eight hex digits from the clock and the process.
pub(crate) fn new_id(prefix: &str) -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{prefix}_{:08x}", (t as u64 ^ (std::process::id() as u64) << 20) as u32)
}

/// Where workspaces live on this machine: `git config --global raea.root`, else default_root.
pub fn root() -> PathBuf {
    let set = config(None, &["--global", "raea.root"]).ok().filter(|s| !s.is_empty());
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    match set {
        Some(s) if s.starts_with("~/") => home.join(&s[2..]),
        Some(s) => PathBuf::from(s),
        None => default_root(&home),
    }
}

/// Where workspaces go with no raea.root set: ~/Studio where workspaces are already (the root before 11-10), else
/// ~/Documents/raea, or ~/raea when iCloud keeps the Documents folder (a workspace's git history and footage don't
/// belong in a synced folder). The feed's host says the same (raea-feed host/workspace.ts defaultRoot).
pub fn default_root(home: &Path) -> PathBuf {
    let studio = home.join("Studio");
    if holds_workspace(&studio) {
        studio
    } else if home.join("Library/Mobile Documents/com~apple~CloudDocs/Documents").is_dir() {
        home.join("raea")
    } else {
        home.join("Documents/raea")
    }
}

/// Whether a folder in `dir` is a workspace (it has .raea/workspace).
fn holds_workspace(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for e in entries.flatten() {
        if e.path().join(".raea/workspace").exists() {
            return true;
        }
    }
    false
}

/// The include in a repo's own config that makes `git config` read a committed file. Idempotent; the path is relative
/// to the config file, which for a submodule lives under the workspace's .git/modules.
pub(crate) fn include(repo: &Path, file: &Path) -> R<()> {
    let cfg = PathBuf::from(git(repo, &["rev-parse", "--path-format=absolute", "--git-path", "config"])?);
    let file = file.canonicalize().map_err(|e| format!("{}: {e}", file.display()))?;
    let rel = relative(cfg.parent().unwrap_or(Path::new("/")), &file);
    let have = config(Some(&cfg), &["--get-all", "include.path"]).unwrap_or_default();
    if have.lines().any(|l| l.trim() == rel) {
        return Ok(());
    }
    config(Some(&cfg), &["--add", "include.path", &rel]).map(drop)
}

fn relative(from: &Path, to: &Path) -> String {
    let (a, b): (Vec<_>, Vec<_>) = (from.components().collect(), to.components().collect());
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out: Vec<String> = std::iter::repeat_n("..".to_string(), a.len() - same).collect();
    out.extend(b[same..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    out.join("/")
}

fn write_new(path: &Path, text: &str) -> R<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A commit of these paths, when they changed: "raea: what".
pub(crate) fn commit(repo: &Path, paths: &[&str], what: &str) -> R<()> {
    let mut add = vec!["add", "--"];
    add.extend(paths);
    git(repo, &add)?;
    if git(repo, &["diff", "--cached", "--quiet"]).is_ok() {
        return Ok(());
    }
    git(repo, &["commit", "-q", "-m", &format!("raea: {what}"), "--"]).map(|_| ())
}

const AGENTS_PROJECT: &str = "# Agents\n\nRun `raea context .` here before any work: the taste, this project's brief.ts and script.ts, its look and the guides\nits modes need, in the order to read them. Specs are TypeScript (`export default brief({...})` from \"raea/brief\").\n\nWhere files go (docs/layout.feature):\n\n- compositions/: what renders, a registerRoot <Composition> or a default export; versions are NAME-v2.tsx\n- components/: what compositions import; every prop is a component whose props animate it\n- controllers/: this project's controllers, over the workspace's shared ones by id\n- assets/: media in (footage, stills, audio, models, fonts) and the brand kit in assets/brand/\n- artifacts/: the small React apps posted to the feed\n- out/: renders\n\nThe workspace's shared/ holds the same five folders for every project: import them as\n`@shared/components/desk`, never by a relative path out of this folder.\n";

/// The kinds of file a project, and the workspace's shared/, keep a folder each for (docs/layout.feature).
pub const KINDS: [&str; 5] = ["compositions", "components", "controllers", "assets", "artifacts"];

/// LFS for media in and renders out, at any depth: in each project and in shared/ (a pattern with a slash is anchored to
/// the workspace root, so the old media/** never matched a project's media/)
const GITATTRIBUTES: &str = "**/assets/** filter=lfs diff=lfs merge=lfs -text\n**/out/** filter=lfs diff=lfs merge=lfs -text\n**/media/** filter=lfs diff=lfs merge=lfs -text\n";

/// A project's tsconfig: @shared/ resolves to the workspace's shared/ for an editor and tsc, as the engine resolves it.
const TSCONFIG_PROJECT: &str = "{\n  \"compilerOptions\": {\n    \"jsx\": \"react-jsx\",\n    \"module\": \"esnext\",\n    \"moduleResolution\": \"bundler\",\n    \"target\": \"es2022\",\n    \"strict\": true,\n    \"noEmit\": true,\n    \"baseUrl\": \".\",\n    \"paths\": { \"@shared/*\": [\"../../shared/*\"] }\n  }\n}\n";

/// `raea workspace …`
pub fn workspace(args: &[String]) -> R<String> {
    let w = words(args);
    match w.first().copied() {
        None => show_workspace(args.iter().any(|a| a == "--json")),
        Some("init") => init(w.get(1).copied(), flag(args, "--layout")),
        Some("adopt") => adopt(Path::new(w.get(1).ok_or(USAGE)?), flag(args, "--name"), flag(args, "--layout")),
        Some(v) => Err(format!("raea workspace has no {v:?}\n{USAGE}")),
    }
}

fn layout_of(l: Option<&str>) -> R<&str> {
    match l.unwrap_or("monorepo") {
        l @ ("monorepo" | "submodules") => Ok(l),
        l => Err(format!("a layout is monorepo or submodules, not {l:?}")),
    }
}

/// `init` in a workspace makes it work after a clone (the includes again, for it and its submodule projects); `init
/// NAME` elsewhere makes a new workspace under raea.root.
fn init(name: Option<&str>, layout: Option<&str>) -> R<String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    if let Some(ws) = up(&here, ".raea/workspace").filter(|_| name.is_none()) {
        include(&ws, &ws.join(".raea/workspace"))?;
        let mut fixed = 0;
        for (_, path, storage) in projects(&ws) {
            let dir = ws.join(&path);
            if storage == "submodule" && dir.join(".git").exists() && dir.join(".raea/project").is_file() {
                include(&dir, &dir.join(".raea/project"))?;
                fixed += 1;
            }
        }
        return Ok(format!("{}: git config reads .raea/workspace{}", ws.display(), match fixed { 0 => String::new(), 1 => " and a submodule project's .raea/project".into(), n => format!(" and {n} submodule projects' .raea/project") }));
    }
    let name = name.ok_or(USAGE)?;
    let dir = root().join(slug(name));
    if dir.join(".raea/workspace").is_file() {
        return Err(format!("{} is a workspace already: `cd` there and `raea workspace init` re-adds its includes", dir.display()));
    }
    make_workspace(&dir, name, layout_of(layout)?)?;
    Ok(format!("{}: workspace {name:?} ({}); next: cd there, raea project new NAME", dir.display(), layout_of(layout)?))
}

/// An existing folder made a workspace in place: git when it isn't a repo, .raea/workspace, and each folder in it
/// holding a brief.ts registered as an inline project. Nothing is moved or deleted.
fn adopt(dir: &Path, name: Option<&str>, layout: Option<&str>) -> R<String> {
    let dir = dir.canonicalize().map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = name.map(String::from).unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    make_workspace(&dir, &name, layout_of(layout)?)?;
    let mut found = vec![];
    for base in [dir.clone(), dir.join("projects")] {
        for e in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let p = e.path();
            if p.join("brief.ts").is_file() && !p.join(".raea/project").is_file() {
                let rel = relative(&dir, &p);
                register(&dir, &p, &rel, "inline")?;
                found.push(rel);
            }
        }
    }
    let mut paths = vec![".raea/workspace"];
    let projs: Vec<String> = found.iter().map(|r| format!("{r}/.raea/project")).collect();
    paths.extend(projs.iter().map(String::as_str));
    commit(&dir, &paths, &format!("adopt {name} as a workspace ({})", count(found.len())))?;
    Ok(format!("{}: workspace {name:?}, {}{}", dir.display(), count(found.len()), if found.is_empty() { String::new() } else { format!(": {}", found.join(", ")) }))
}

fn count(n: usize) -> String {
    if n == 1 { "1 project".into() } else { format!("{n} projects") }
}

pub(crate) fn make_workspace(dir: &Path, name: &str, layout: &str) -> R<()> {
    // never a workspace in another repo's own folder (on a case-insensitive disk ~/Raea is ~/raea, raea's checkout); a
    // repo further up, as a home folder kept in git, is fine: the workspace is a repo of its own inside it
    let parent = dir.parent().unwrap_or(Path::new("/"));
    if !dir.join(".git").exists() && parent.exists() {
        let top = git(parent, &["rev-parse", "--show-toplevel"]).ok().map(PathBuf::from);
        if top.is_some_and(|t| t.canonicalize().ok() == parent.canonicalize().ok()) {
            return Err(format!("{} would be a folder of the git repo {}: set where workspaces go, `git config --global raea.root ~/Documents/raea`", dir.display(), parent.display()));
        }
    }
    std::fs::create_dir_all(dir.join("projects")).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !dir.join(".git").exists() {
        git(dir, &["init", "-q"])?;
    }
    let file = dir.join(".raea/workspace");
    if get(&file, "workspace.id").is_none() {
        set(&file, "workspace.id", &new_id("ws"))?;
    }
    set(&file, "workspace.name", name)?;
    set(&file, "workspace.layout", layout)?;
    include(dir, &file)?;
    // what every project here can call on (docs/layout.feature LY1, LY2): imported as @shared/<kind>/...
    for k in KINDS {
        write_new(&dir.join("shared").join(k).join(".gitkeep"), "")?;
    }
    write_new(&dir.join("projects/.gitkeep"), "")?;
    write_new(&dir.join("taste.ts"), &format!("import {{ taste }} from \"raea/brief\";\n\n// {name}'s house rules: every project here follows them, after the person's own taste.\nexport default taste({{\n  title: {:?},\n  sections: [],\n}});\n", format!("{name}: house rules")))?;
    write_new(&dir.join(".gitattributes"), GITATTRIBUTES)?;
    let kept: Vec<String> = KINDS.iter().map(|k| format!("shared/{k}/.gitkeep")).collect();
    let mut files: Vec<&str> = vec![".raea/workspace", "projects/.gitkeep", "taste.ts", ".gitattributes"];
    files.extend(kept.iter().map(String::as_str));
    commit(dir, &files, &format!("workspace {name}"))
}

/// The workspace's projects: (name, path, storage), from .raea/workspace.
pub fn projects(ws: &Path) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = vec![];
    for (k, v) in all(&ws.join(".raea/workspace")) {
        let Some(rest) = k.strip_prefix("project.") else { continue };
        let Some((name, key)) = rest.rsplit_once('.') else { continue };
        let at = out.iter().position(|p| p.0 == name).unwrap_or_else(|| {
            out.push((name.to_string(), format!("projects/{name}"), "inline".into()));
            out.len() - 1
        });
        match key {
            "path" => out[at].1 = v,
            "storage" => out[at].2 = v,
            _ => {}
        }
    }
    out
}

/// A project's own .raea/project, and its line in the workspace's.
fn register(ws: &Path, dir: &Path, rel: &str, storage: &str) -> R<String> {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let file = dir.join(".raea/project");
    let id = get(&file, "project.id").unwrap_or_else(|| new_id("pj"));
    set(&file, "project.id", &id)?;
    set(&file, "project.name", &name)?;
    let wsf = ws.join(".raea/workspace");
    set(&wsf, &format!("project.{name}.path"), rel)?;
    set(&wsf, &format!("project.{name}.id"), &id)?;
    set(&wsf, &format!("project.{name}.storage"), storage)?;
    Ok(id)
}

fn show_workspace(json_out: bool) -> R<String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let ws = up(&here, ".raea/workspace").ok_or("not in a workspace: raea workspace init NAME makes one (under `git config --global raea.root`, else ~/Documents/raea)")?;
    let file = ws.join(".raea/workspace");
    let ps = projects(&ws);
    if json_out {
        let projects: Vec<_> = ps.iter().map(|(n, p, s)| json!({"name": n, "path": p, "storage": s, "id": get(&file, &format!("project.{n}.id"))})).collect();
        return Ok(json!({"path": ws, "id": get(&file, "workspace.id"), "name": get(&file, "workspace.name"), "layout": get(&file, "workspace.layout"), "projects": projects}).to_string());
    }
    let mut out = format!("{} ({}, {})\n{}\n", get(&file, "workspace.name").unwrap_or_default(), get(&file, "workspace.id").unwrap_or_default(), get(&file, "workspace.layout").unwrap_or_default(), ws.display());
    for (n, p, s) in &ps {
        out.push_str(&format!("  {n}  {p}  {s}\n"));
    }
    Ok(out.trim_end().to_string())
}

/// `raea project …`
pub fn project(args: &[String]) -> R<String> {
    let w = words(args);
    match w.first().copied() {
        None => show_project(args.iter().any(|a| a == "--json")),
        Some("new") => new_project(w.get(1).ok_or(USAGE)?, flag(args, "--storage")),
        Some("split") => storage::split(w.get(1).ok_or(USAGE)?, flag(args, "--remote")),
        Some("join") => storage::join(w.get(1).ok_or(USAGE)?),
        // PJ8: the folder's compositions, components and controllers, and what isn't connected
        Some("scan") => {
            let dir = w.get(1).map(PathBuf::from).map_or_else(|| std::env::current_dir().map_err(|e| e.to_string()), Ok)?;
            let r = crate::registry::scan(&dir);
            Ok(if args.iter().any(|a| a == "--json") { r.json().to_string() } else { r.text() })
        }
        Some(v) => Err(format!("raea project has no {v:?}\n{USAGE}")),
    }
}

fn new_project(name: &str, storage: Option<&str>) -> R<String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let ws = up(&here, ".raea/workspace").ok_or("not in a workspace: raea workspace init NAME makes one")?;
    new_project_in(&ws, name, storage)
}

/// A new project in the workspace at `ws` (the folder holding .raea/workspace).
pub(crate) fn new_project_in(ws: &Path, name: &str, storage: Option<&str>) -> R<String> {
    let ws = ws.to_path_buf();
    let slugged = slug(name);
    if slugged.is_empty() {
        return Err(format!("{name:?} has no letters or digits to name a folder by"));
    }
    let rel = format!("projects/{slugged}");
    let dir = ws.join(&rel);
    if dir.join(".raea/project").is_file() {
        return Err(format!("{} is a project already", dir.display()));
    }
    let layout = get(&ws.join(".raea/workspace"), "workspace.layout").unwrap_or_else(|| "monorepo".into());
    let storage = match storage.unwrap_or(if layout == "submodules" { "submodule" } else { "inline" }) {
        s @ ("inline" | "submodule") => s,
        s => return Err(format!("a project's storage is inline or submodule, not {s:?}")),
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    write_new(&dir.join("brief.ts"), &format!("import {{ brief }} from \"raea/brief\";\n\nexport default brief({{\n  title: {name:?},\n}});\n"))?;
    write_new(&dir.join("AGENTS.md"), AGENTS_PROJECT)?;
    write_new(&dir.join("tsconfig.json"), TSCONFIG_PROJECT)?;
    // the project's layout (docs/layout.feature LY1): a folder for each kind of file, and its renders
    for d in KINDS.iter().chain(["out"].iter()) {
        write_new(&dir.join(d).join(".gitkeep"), "")?;
    }
    let id = register(&ws, &dir, &rel, storage)?;
    let kept: Vec<String> = KINDS.iter().chain(["out"].iter()).map(|d| format!("{d}/.gitkeep")).collect();
    let mut files: Vec<&str> = vec![".raea/project", "brief.ts", "AGENTS.md", "tsconfig.json"];
    files.extend(kept.iter().map(String::as_str));
    if storage == "submodule" {
        git(&dir, &["init", "-q"])?;
        commit(&dir, &files, &format!("project {name}"))?;
        // the project's own repo, added where it stands; its url is the folder until it has a remote of its own
        git(&ws, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", &format!("./{rel}"), &rel])?;
        include(&dir, &dir.join(".raea/project"))?;
        commit(&ws, &[".raea/workspace", ".gitmodules", &rel], &format!("project {name} (submodule)"))?;
    } else {
        let paths: Vec<String> = files.iter().map(|f| format!("{rel}/{f}")).collect();
        let mut all_paths: Vec<&str> = vec![".raea/workspace"];
        all_paths.extend(paths.iter().map(String::as_str));
        commit(&ws, &all_paths, &format!("project {name}"))?;
    }
    Ok(format!("{}: project {name:?} ({id}, {storage}); next: shape brief.ts, then raea context .", dir.display()))
}

fn show_project(json_out: bool) -> R<String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let dir = up(&here, ".raea/project").ok_or("not in a project: raea project new NAME (in a workspace) makes one")?;
    let file = dir.join(".raea/project");
    let ws = up(&dir, ".raea/workspace").filter(|w| *w != dir);
    let overrides: Vec<(String, String)> = all(&file).into_iter().filter(|(k, _)| k.starts_with("controller.")).collect();
    if json_out {
        let o: serde_json::Map<String, serde_json::Value> = overrides.iter().map(|(k, v)| (k.trim_start_matches("controller.").to_string(), json!(v))).collect();
        return Ok(json!({"path": dir, "id": get(&file, "project.id"), "name": get(&file, "project.name"), "workspace": ws, "controllers": o}).to_string());
    }
    let mut out = format!("{} ({})\n{}\n", get(&file, "project.name").unwrap_or_default(), get(&file, "project.id").unwrap_or_default(), dir.display());
    if let Some(ws) = &ws {
        out.push_str(&format!("in workspace {}\n", ws.display()));
    }
    for (k, v) in overrides {
        out.push_str(&format!("  {} = {v}\n", k.trim_start_matches("controller.")));
    }
    Ok(out.trim_end().to_string())
}
