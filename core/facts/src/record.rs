//! The project's record (docs/direction.feature DR3): what each render and each `raea check` found, one JSON file
//! apiece under the project's .raea/record/, with the commit the project was at. `raea findings` reads it back across
//! renders (what was fixed, what is new, what is carried) and `raea next` reads what stands. A file outside a raea
//! project (no .raea/project at or above it) has no record.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Finding;

/// The most entries a project keeps; the oldest go first.
const KEEP: usize = 500;

/// What `--despite` let through, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Despite {
    pub check: String,
    pub why: String,
}

/// One render's or check's findings, as the record keeps them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// its name in the record: when (UTC) and of what, `20261110-161530-edit`
    pub id: String,
    /// seconds since 1970, for order
    pub at: u64,
    /// "raea render" or "raea check"
    pub by: String,
    /// what was rendered (the composition) or checked (the video), relative to the project
    pub of: String,
    /// what it wrote, relative to the project when it's in it
    #[serde(default)]
    pub out: Option<String>,
    /// what it is set against others of: "video", "sound", "still", "frames", "review" or "check" (kind())
    #[serde(default)]
    pub kind: String,
    /// the project's commit, "+" after it when the file it's of has changes not committed (None: no git repository)
    #[serde(default)]
    pub commit: Option<String>,
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub despite: Vec<Despite>,
}

/// The nearest folder at or above `from` that holds `marker` (.raea/workspace, .raea/project).
pub fn up(from: &Path, marker: &str) -> Option<PathBuf> {
    let from = from.canonicalize().ok()?;
    from.ancestors().find(|d| d.join(marker).is_file()).map(Path::to_path_buf)
}

/// A path made absolute through its nearest folder that exists (an output may not be written yet).
fn real(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    match (p.parent().filter(|d| !d.as_os_str().is_empty()), p.file_name()) {
        (Some(d), Some(n)) => real(d).join(n),
        _ => std::env::current_dir().map(|c| c.join(p)).unwrap_or_else(|_| p.to_path_buf()),
    }
}

/// A path as the record names it: relative to the project when it's in it.
pub fn rel(project: &Path, p: &Path) -> String {
    let full = real(p);
    full.strip_prefix(project).map(|r| r.display().to_string()).unwrap_or_else(|_| full.display().to_string())
}

/// The project's commit, with "+" when `of` (a path from the project; "." for all of it) differs from it.
pub fn commit(project: &Path, of: &str) -> Option<String> {
    let git = |a: &[&str]| {
        let o = std::process::Command::new("git").arg("-C").arg(project).args(a).output().ok()?;
        o.status.success().then_some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let head = git(&["rev-parse", "--short", "HEAD"])?;
    let changed = git(&["status", "--porcelain", "--", of]).is_some_and(|s| !s.is_empty());
    Some(if changed { format!("{head}+") } else { head })
}

/// What a run wrote, as what its entries are set against: a check, a review (a folder holding review.json), a video, a
/// sound, a still or a folder of frames. A review's tiles against the video before it would read as fixed.
pub fn kind(by: &str, out: Option<&Path>) -> String {
    let ext = out.and_then(Path::extension).map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    let k = match ext.as_str() {
        _ if by == "raea check" => "check",
        _ if out.is_none() => "run",
        _ if out.is_some_and(|o| o.join("review.json").is_file()) => "review",
        "mp4" | "mov" | "webm" | "mkv" => "video",
        "wav" => "sound",
        "png" | "jpg" | "jpeg" => "still",
        _ => "frames",
    };
    k.into()
}

/// What `by` found of `file` (writing `out`) kept in its project's record, named by `stamp` (UTC, 20261110-161530)
/// and the file's name: the entry, or None outside a project.
pub fn keep(by: &str, file: &Path, out: Option<&Path>, findings: &[Finding], despite: Vec<Despite>, stamp: &str) -> Option<Entry> {
    // made absolute first: a bare name's parent is "", which names no folder
    let project = up(real(file).parent()?, ".raea/project")?;
    let dir = project.join(".raea/record");
    std::fs::create_dir_all(&dir).ok()?;
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let of = rel(&project, file);
    let base = format!("{stamp}-{}", file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    let id = (1..100).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|id| !dir.join(format!("{id}.json")).exists())?;
    let entry = Entry { id: id.clone(), at, by: by.into(), commit: commit(&project, &of), of, out: out.map(|o| rel(&project, o)), kind: kind(by, out), findings: findings.to_vec(), despite };
    std::fs::write(dir.join(format!("{id}.json")), serde_json::to_vec_pretty(&entry).ok()?).ok()?;
    // the oldest past KEEP go (the names sort by time)
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    names.sort();
    for p in names.iter().take(names.len().saturating_sub(KEEP)) {
        let _ = std::fs::remove_file(p);
    }
    Some(entry)
}

/// The project's record, oldest first.
pub fn read(project: &Path) -> Vec<Entry> {
    let mut v: Vec<Entry> = std::fs::read_dir(project.join(".raea/record")).into_iter().flatten().flatten().filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()).collect();
    v.sort_by(|a, b| (a.at, &a.id).cmp(&(b.at, &b.id)));
    v
}

/// A finding as the same finding in another render: its check and what it is about (its words when it names
/// nothing), not its frames, which move as the piece is cut.
pub fn same(f: &Finding) -> String {
    let about: Vec<&str> = f.about.iter().map(|a| a.key.as_deref().unwrap_or(&a.name)).collect();
    if about.is_empty() { format!("{} {}", f.check, f.says) } else { format!("{} {}", f.check, about.join(", ")) }
}

/// Between two entries of one thing: what was fixed (found before, not now), what is new, what is carried, and what
/// was renamed (before, now): gone under one name and new under another with the same check, frames and measure.
pub struct Changes<'a> {
    pub fixed: Vec<&'a Finding>,
    pub new: Vec<&'a Finding>,
    pub carried: Vec<&'a Finding>,
    pub renamed: Vec<(&'a Finding, &'a Finding)>,
}

pub fn changes<'a>(before: &'a [Finding], now: &'a [Finding]) -> Changes<'a> {
    let has = |fs: &[Finding], f: &Finding| fs.iter().any(|g| same(g) == same(f));
    let mut fixed: Vec<&Finding> = before.iter().filter(|f| !has(now, f)).collect();
    let mut renamed = Vec::new();
    // a part renamed is no fix: the same finding, measured the same on the same frames, under a new name
    let new = now.iter().filter(|f| !has(before, f)).filter(|n| match fixed.iter().position(|g| g.check == n.check && g.frames == n.frames && g.measure.is_some() && g.measure == n.measure) {
        Some(at) => {
            renamed.push((fixed.remove(at), *n));
            false
        }
        None => true,
    });
    let new = new.collect();
    Changes { fixed, new, carried: now.iter().filter(|f| has(before, f)).collect(), renamed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn a_render_is_kept_in_its_projects_record_and_compared_with_the_one_before() {
        let root = std::env::temp_dir().join(format!("raea-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".raea")).unwrap();
        std::fs::write(root.join(".raea/project"), "").unwrap();
        std::fs::write(root.join("edit.tsx"), "").unwrap();
        let clips = Finding::new("clips", Level::Error, "rider in seat", "raea render").about("rider", None).about("seat", None).frames(10, 20);
        let tilt = Finding::new("tilted", Level::Warn, "vase leans", "raea render").about("vase", None);
        let a = keep("raea render", &root.join("edit.tsx"), Some(&root.join("out/edit.mp4")), &[clips.clone(), tilt.clone()], vec![], "20261110-160000").unwrap();
        // the same clip on other frames is the same finding
        let b = keep("raea render", &root.join("edit.tsx"), None, &[clips.frames(12, 22)], vec![], "20261110-160000").unwrap();
        assert_eq!((a.id.as_str(), b.id.as_str(), a.of.as_str(), a.out.as_deref()), ("20261110-160000-edit", "20261110-160000-edit-2", "edit.tsx", Some("out/edit.mp4")));
        // what each run wrote is the kind it's set against: the video, frames, a review's folder, a check
        std::fs::create_dir_all(root.join("review")).unwrap();
        std::fs::write(root.join("review/review.json"), "{}").unwrap();
        let k = |by: &str, o: &str| kind(by, Some(&root.join(o)));
        assert_eq!((a.kind.as_str(), b.kind.as_str(), k("raea render", "review"), k("raea render", "frames/"), k("raea check", "out/edit.mp4")), ("video", "run", "review".into(), "frames".into(), "check".into()));
        let all = read(&root);
        assert_eq!(all.len(), 2);
        let ch = changes(&all[0].findings, &all[1].findings);
        assert_eq!((ch.fixed.len(), ch.new.len(), ch.carried.len()), (1, 0, 1));
        assert_eq!(ch.fixed[0].check, "tilted");
        // outside a project, nothing is kept
        assert!(keep("raea render", &std::env::temp_dir().join("loose.tsx"), None, &[], vec![], "x").is_none());
        // a video copied from the export cache says what was found when it was drawn (finding::keep_to, replay)
        let _held = crate::finding::SAYING.lock().unwrap_or_else(|e| e.into_inner());
        crate::finding::said(std::slice::from_ref(&tilt));
        assert!(crate::finding::keep_to(&root.join("cached.findings")));
        let _ = crate::finding::taken();
        crate::finding::replay(&root.join("cached.findings"));
        assert_eq!(crate::finding::taken(), vec![tilt]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
