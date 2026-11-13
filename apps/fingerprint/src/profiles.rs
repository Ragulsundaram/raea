//! `raea fingerprint profile REFS... --as NAME [--against LIST]` and `score FILE --profile NAME` (docs/fingerprint.feature
//! FP6e): a profile's measures taken from work the person admires, kept in the fingerprint's profiles/NAME.json, and a
//! piece critiqued against it, measure by measure, with where and which way to move.

use crate::{flag, words};
use anyhow::{Context, Result, anyhow, bail};
use raea_sys_columns::{Encoders, media};
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint_use::profile::{Measures, NAMES, Out, Profile, critique, measures, profile};
use raea_sys_fingerprint_use::score::clock;
use std::path::{Path, PathBuf};

const BINS: usize = 12;
/// The windows the critique says "where" with, seconds
const WIN: f64 = 4.0;

fn at(cwd: &Path, p: &str) -> PathBuf {
    if Path::new(p).is_absolute() { PathBuf::from(p) } else { cwd.join(p) }
}

/// A piece's measures, whole and in windows
pub(crate) fn measured(enc: &mut Encoders, file: &Path) -> Result<(Measures, Vec<((f64, f64), Measures)>)> {
    let c = enc.columns(file)?;
    let pcm = media::sound(file, 16_000).unwrap_or_default();
    let cols = ((c.dur * 10.0) as usize).max(1);
    let spec = media::spectrogram(&pcm, 16_000, cols, BINS);
    let whole = measures(&c, &spec, BINS, 0.0, c.dur);
    let n = (c.dur / WIN).ceil() as usize;
    let windows = (0..n).map(|i| {
        let (a, b) = (i as f64 * WIN, ((i + 1) as f64 * WIN).min(c.dur));
        ((a, b), measures(&c, &spec, BINS, a, b))
    }).collect();
    Ok((whole, windows))
}

fn path(dir: &Folder, name: &str) -> PathBuf {
    dir.dir.join("profiles").join(format!("{name}.json"))
}

pub(crate) fn make(enc: &mut Encoders, dir: &Folder, args: &[String], cwd: &Path) -> Result<String> {
    let name = flag(args, "--as").ok_or_else(|| anyhow!("name it: raea fingerprint profile REFS... --as NAME"))?;
    let refs: Vec<PathBuf> = words(args).map_err(|e| anyhow!(e))?.iter().map(|f| at(cwd, f)).collect();
    if refs.len() < 2 {
        bail!("a profile needs two references or more (three for its leave-one-out check)");
    }
    let mut r = Vec::new();
    for f in &refs {
        r.push((f.display().to_string(), measured(enc, f)?.0));
    }
    let mut other = Vec::new();
    if let Some(list) = flag(args, "--against") {
        let list = at(cwd, list);
        for l in std::fs::read_to_string(&list).with_context(|| list.display().to_string())?.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            other.push(measured(enc, &at(list.parent().unwrap_or(cwd), l))?.0);
        }
    }
    let p = profile(name, r, &other);
    std::fs::create_dir_all(dir.dir.join("profiles"))?;
    std::fs::write(path(dir, name), serde_json::to_vec_pretty(&p)?)?;
    let mut out = format!("profile {name} from {} references:\n", p.refs.len());
    for (k, words) in NAMES {
        if let Some((lo, hi)) = p.ranges.get(k) {
            out += &format!("  {words:40} {lo:.2} to {hi:.2}\n");
        }
    }
    out += &match (p.inside_own, p.inside_other) {
        (Some(o), Some(x)) => format!("leave-one-out: a reference falls in the others' ranges {:.0}% of the time, unrelated work {:.0}%: {}\n", o * 100.0, x * 100.0, if p.tells == Some(true) { "it tells this taste apart" } else { "it does NOT tell this taste apart" }),
        (Some(o), None) => format!("leave-one-out: a reference falls in the others' ranges {:.0}% of the time (give --against LIST to compare with unrelated work)\n", o * 100.0),
        _ => "leave-one-out needs three references\n".into(),
    };
    Ok(out)
}

/// The critique of a piece against a saved profile, as lines
pub(crate) fn against(enc: &mut Encoders, dir: &Folder, name: &str, file: &Path) -> Result<(String, Vec<Out>)> {
    let p: Profile = serde_json::from_slice(&std::fs::read(path(dir, name)).with_context(|| format!("no profile {name} (raea fingerprint profile REFS... --as {name})"))?)?;
    let (whole, windows) = measured(enc, file)?;
    let out = critique(&p, &whole, &windows);
    let words = |k: &str| NAMES.iter().find(|n| n.0 == k).map_or(k.to_string(), |n| n.1.to_string());
    let mut t = format!("against the {name} profile: {}\n", if out.is_empty() { "every measure in range".to_string() } else { format!("{} out of range", out.len()) });
    for o in &out {
        t += &format!("  {} {:.2} is outside {:.2} to {:.2}: {} it{}\n", words(&o.measure), o.value, o.range.0, o.range.1, o.move_, o.at.map_or(String::new(), |(a, _)| format!(", most at {}", clock(a))));
    }
    Ok((t, out))
}
