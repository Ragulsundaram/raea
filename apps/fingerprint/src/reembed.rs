//! Encoder upgrades keep the evidence (docs/fingerprint.feature FP8c). When this machine's encoders aren't the ones a
//! fingerprint was trained on, `raea fingerprint reembed PLACE...` re-embeds every row whose media it finds under
//! those files and folders (matched by content hash), moves the fingerprint to the new encoders and retrains. A row
//! whose media isn't found keeps its old pin and trains no head until it's re-embedded or labelled again; a bootstrap's
//! damaged rows are remade by running the bootstrap again.

use crate::words;
use anyhow::{Result, anyhow};
use raea_sys_columns::{Encoders, media};
use raea_sys_fingerprint::evidence::features;
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint::taxonomy::Fingerprint;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MEDIA: [&str; 9] = ["mp4", "mov", "m4v", "webm", "mkv", "wav", "mp3", "m4a", "flac"];

/// An encoder set's short name, as a row's pin
pub(crate) fn pin_of(p: &BTreeMap<String, String>) -> String {
    p.iter().map(|(k, v)| format!("{k}:{}", &v[..v.len().min(8)])).collect::<Vec<_>>().join(",")
}

/// The media files under each place, two folders deep
fn files(places: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(d: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() && depth > 0 {
                walk(&p, depth - 1, out);
            } else if MEDIA.iter().any(|m| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(m))) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for p in places {
        if p.is_dir() { walk(p, 2, &mut out) } else { out.push(p.clone()) }
    }
    out
}

/// Rows moved to `here`'s encoders where their media is found; returns the fingerprint as it is now and what happened
pub(crate) fn reembed(enc: &mut Encoders, dir: &Folder, mut fp: Fingerprint, here: BTreeMap<String, String>, args: &[String], cwd: &Path) -> Result<(Fingerprint, String)> {
    let places: Vec<PathBuf> = words(args).map_err(|e| anyhow!(e))?.iter().map(|f| cwd.join(f)).collect();
    let mut l = dir.ledger()?;
    if fp.encoders != here {
        let old = pin_of(&fp.encoders);
        l.rows.iter_mut().filter(|r| r.pin.is_none()).for_each(|r| r.pin = Some(old.clone()));
        fp.encoders = here;
        dir.rewrite(&fp)?;
    }
    let waiting: std::collections::BTreeSet<String> = l.rows.iter().filter(|r| r.pin.is_some() && !matches!(r.source.as_str(), "damage" | "cheat")).map(|r| r.media.clone()).collect();
    let mut done = 0;
    for f in files(&places) {
        if waiting.is_empty() {
            break;
        }
        let Ok(sha) = media::sha256(&f) else { continue };
        if !waiting.contains(&sha) {
            continue;
        }
        let c = enc.columns(&f)?;
        for i in 0..l.rows.len() {
            let r = &l.rows[i];
            if r.media == sha && r.pin.is_some() && !matches!(r.source.as_str(), "damage" | "cheat") {
                let x = features(&c, r.a, r.b);
                l.set_features(i, &x);
                l.rows[i].pin = None;
                done += 1;
            }
        }
    }
    dir.write_ledger(&l)?;
    let left = l.rows.iter().filter(|r| r.pin.is_some()).count();
    let damage = l.rows.iter().filter(|r| r.pin.is_some() && matches!(r.source.as_str(), "damage" | "cheat")).count();
    let mut out = format!("re-embedded {done} rows on this machine's encoders; {left} wait on their old ones");
    if damage > 0 {
        out += &format!(" ({damage} from a bootstrap: run raea fingerprint bootstrap again to remake them)");
    }
    Ok((fp, out + "\n"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_pin_names_each_encoder_and_its_hash() {
        let p = std::collections::BTreeMap::from([("clap".to_string(), "0123456789abcdef".to_string()), ("siglip".to_string(), "fedcba".to_string())]);
        assert_eq!(super::pin_of(&p), "clap:01234567,siglip:fedcba");
    }
}
