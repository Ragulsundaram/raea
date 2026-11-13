//! Which fingerprint judges a piece (docs/fingerprint.feature FP14c): `raea fingerprint which PATH` says the one
//! chosen, what it's for and why; `raea fingerprint use NAME [--in project|workspace] [PATH]` pins one by writing that
//! level's `default`. score and teach use the chosen one unless --fp names another. No encoders are needed.

use crate::{flag, has, words};
use raea_sys_fingerprint::folder::{Folder, root};
use raea_sys_fingerprint_use::which::{Choice, default_file, levels, piece, which};
use std::path::{Path, PathBuf};

/// The fingerprint a verb uses: --fp, else the one that judges `file`, else the account's default, else "me"
pub(crate) fn chosen(args: &[String], file: Option<&Path>) -> (Folder, Option<Choice>) {
    if let Some(n) = flag(args, "--fp") {
        return (Folder::named(n), None);
    }
    if let Some(c) = file.and_then(which) {
        return (Folder { dir: c.dir.clone() }, Some(c));
    }
    let n = std::fs::read_to_string(root().join("default")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "me".into());
    (Folder::named(&n), None)
}

fn here(cwd: &Path, args: &[String], at: usize) -> PathBuf {
    let w = words(args).unwrap_or_default();
    w.get(at).map_or_else(|| cwd.to_path_buf(), |p| cwd.join(p))
}

pub(crate) fn which_here(args: &[String], cwd: &Path) -> i32 {
    let path = here(cwd, args, 0);
    let Some(c) = which(&path) else {
        eprintln!("raea fingerprint which: you have no fingerprint yet (`raea fingerprint new me` makes one)");
        return 1;
    };
    if has(args, "--json") {
        let p = piece(&path);
        println!("{}", serde_json::json!({"chosen": c, "piece": p, "looked_in": levels(&p).iter().map(|(l, d)| serde_json::json!({"level": l, "dir": d})).collect::<Vec<_>>()}));
    } else {
        println!("{}  {}", c.name, c.about.as_deref().unwrap_or(""));
        println!("  why: {}", c.why);
        println!("  at: {}", c.dir.display());
    }
    0
}

pub(crate) fn use_here(args: &[String], cwd: &Path) -> i32 {
    let Some(name) = words(args).ok().and_then(|w| w.first().map(|s| s.to_string())) else {
        eprintln!("raea fingerprint use: which one? raea fingerprint use NAME [--in project|workspace] [PATH]");
        return 2;
    };
    let level = flag(args, "--in").unwrap_or("account");
    if !matches!(level, "project" | "workspace" | "account") {
        eprintln!("raea fingerprint use: --in takes project or workspace, not {level}");
        return 2;
    }
    let path = here(cwd, args, 1);
    let Some(file) = default_file(&path, level) else {
        eprintln!("raea fingerprint use: {} isn't in a {level} (no .raea/{level} above it)", path.display());
        return 1;
    };
    let p = piece(&path);
    let all = levels(&p);
    let from = all.iter().position(|(l, _)| *l == level).unwrap_or(0);
    if !all[from..].iter().any(|(_, d)| Folder { dir: d.join(&name) }.exists()) {
        eprintln!("raea fingerprint use: no fingerprint {name} at the {level} or above it");
        return 1;
    }
    let wrote = std::fs::create_dir_all(file.parent().unwrap_or(cwd)).and(std::fs::write(&file, &name));
    if let Err(e) = wrote {
        eprintln!("raea fingerprint use: {}: {e}", file.display());
        return 1;
    }
    println!("{name} judges this {level}'s pieces now ({})", file.display());
    0
}

/// `raea fingerprint diff A B [--json]` (FP8b): the axes two fingerprints rank the same evidence most differently on
pub(crate) fn diff_here(args: &[String]) -> i32 {
    let w = words(args).unwrap_or_default();
    let [a, b] = w.as_slice() else {
        eprintln!("raea fingerprint diff: which two? raea fingerprint diff A B");
        return 2;
    };
    match raea_sys_fingerprint_use::diff::diff(&Folder::named(a), &Folder::named(b), 3) {
        Ok(d) if has(args, "--json") => {
            println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default());
            0
        }
        Ok(d) => {
            println!("where {a} and {b} differ, most apart first (0 agree, 1 unrelated, 2 opposed):");
            for x in &d {
                println!("  {:16} {:.2} over {} windows", x.axis, x.apart, x.windows);
                for m in &x.moments {
                    println!("    {} {:.1}-{:.1}s: {a} ranks it {:.0}%, {b} {:.0}%", &m.media[..m.media.len().min(12)], m.a, m.b, m.rank_a * 100.0, m.rank_b * 100.0);
                }
            }
            0
        }
        Err(e) => {
            eprintln!("raea fingerprint diff: {e:#}");
            1
        }
    }
}

/// `raea fingerprint share NAME --in project|workspace [PATH]` (FP8a): the fingerprint's taxonomy, words and evidence
/// copied into that level's .raea/fingerprints, to travel with its repo; never its heads (each machine retrains them)
/// and never media (the rows hold embeddings, spans and labels)
pub(crate) fn share_here(args: &[String], cwd: &Path) -> i32 {
    let Some(name) = words(args).ok().and_then(|w| w.first().map(|s| s.to_string())) else {
        eprintln!("raea fingerprint share: which one? raea fingerprint share NAME --in project|workspace [PATH]");
        return 2;
    };
    let level = flag(args, "--in").unwrap_or("project");
    let path = here(cwd, args, 1);
    let Some(to) = default_file(&path, level).and_then(|f| f.parent().map(|d| d.join(&name))) else {
        eprintln!("raea fingerprint share: {} isn't in a {level} (no .raea/{level} above it)", path.display());
        return 1;
    };
    let from = Folder::named(&name);
    let run = || -> anyhow::Result<usize> {
        from.read(None)?;
        let dest = Folder { dir: to.clone() };
        if dest.exists() {
            anyhow::bail!("{} has a fingerprint {name} already", to.display());
        }
        std::fs::create_dir_all(&to)?;
        for f in ["fingerprint.ts", "fingerprint.json", "words.json", "words.f16"] {
            if from.dir.join(f).is_file() {
                std::fs::copy(from.dir.join(f), to.join(f))?;
            }
        }
        let l = from.ledger()?;
        dest.write_ledger(&l)?;
        Ok(l.rows.len())
    };
    match run() {
        Ok(n) => {
            println!("shared {name} into {} with {n} evidence rows, no heads and no media; commit it with the {level}", to.display());
            0
        }
        Err(e) => {
            eprintln!("raea fingerprint share: {e:#}");
            1
        }
    }
}
