//! A shared fingerprint (FP8a): one kept in a project's or workspace's .raea/fingerprints travels with that folder's
//! repo, so sharing is evidence, never weights or media. Its rows are kept a folder a person (evidence/WHO/), so two
//! teammates labelling at once never edit the same file, and every machine retrains the heads from all of them
//! (heads.safetensors and report.json stay on the machine: the folder's .gitignore says so). A person's own
//! fingerprint in ~/.raea keeps one evidence.jsonl.

use crate::evidence::Ledger;
use anyhow::Result;
use std::path::Path;

/// Whether a fingerprint folder is a project's or workspace's: PLACE/.raea/fingerprints/NAME
pub fn shared(dir: &Path) -> bool {
    let Some(raea) = dir.parent().filter(|p| p.file_name().is_some_and(|n| n == "fingerprints")).and_then(Path::parent) else { return false };
    raea.file_name().is_some_and(|n| n == ".raea") && (raea.join("project").is_file() || raea.join("workspace").is_file())
}

/// Every person's rows, read together
pub fn read(dir: &Path) -> Result<Ledger> {
    let mut l = Ledger::read(dir)?;
    let mut people: Vec<_> = std::fs::read_dir(dir.join("evidence")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    people.sort();
    for p in people {
        let one = Ledger::read(&p)?;
        for i in 0..one.rows.len() {
            l.push(one.rows[i].clone(), one.features(i));
        }
    }
    Ok(l)
}

/// Each person's rows to their own folder
pub fn write(dir: &Path, l: &Ledger) -> Result<()> {
    let mut who: Vec<&str> = l.rows.iter().map(|r| r.who.as_str()).collect();
    who.sort();
    who.dedup();
    for w in who {
        let mut one = Ledger::default();
        for i in (0..l.rows.len()).filter(|&i| l.rows[i].who == w) {
            one.push(l.rows[i].clone(), l.features(i));
        }
        let safe: String = w.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        one.write(&dir.join("evidence").join(safe))?;
    }
    for f in ["evidence.jsonl", "evidence.f16"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    std::fs::write(dir.join(".gitignore"), "# made on each machine from the evidence (docs/fingerprint.feature FP8a)\nheads.safetensors\nheads.part\nreport.json\n")?;
    Ok(())
}
