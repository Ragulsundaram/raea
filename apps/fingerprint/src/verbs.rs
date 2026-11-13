//! The verbs, run with the encoders loaded: new, bootstrap, teach and score; and report, which needs no encoders.

use crate::{Eval, flag, has, words};
use anyhow::{Context, Result, anyhow, bail};
use raea_sys_columns::{Encoders, FPS, media};
use raea_sys_fingerprint::evidence::{Ledger, Row, features};
use raea_sys_fingerprint::folder::{Folder, root};
use raea_sys_fingerprint::score::{Scored, clock, score, summary, timeline};
use raea_sys_fingerprint::sheet::{Tick, sheet};
use raea_sys_fingerprint::taxonomy::{Fingerprint, base};
use raea_sys_fingerprint::words::Words;
use crate::bootstrap;
use raea_sys_fingerprint::{report_text, retrain};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The fingerprint a verb is about: --fp, else the one `new` made first, else "me"
fn name(args: &[String]) -> String {
    flag(args, "--fp").map(str::to_string).or_else(|| std::fs::read_to_string(root().join("default")).ok().map(|s| s.trim().to_string())).unwrap_or_else(|| "me".into())
}

fn at(cwd: &Path, p: &str) -> PathBuf {
    let p = Path::new(p);
    if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) }
}

/// A verb's answer and exit code
pub(crate) fn run(enc: &mut Encoders, args: &[String], cwd: &Path, eval: Eval, say: &mut dyn FnMut(String)) -> (String, i32) {
    let rest = &args[1..];
    let ran = match args[0].as_str() {
        "new" => new(enc, rest),
        "bootstrap" => boot(enc, rest, cwd, eval, say),
        "teach" => teach(enc, rest, cwd, eval),
        "score" => scored(enc, rest, cwd, eval),
        v => Err(anyhow!("no verb {v}")),
    };
    ran.unwrap_or_else(|e| (format!("raea fingerprint {}: {e:#}", args[0]), 1))
}

fn pins(enc: &Encoders) -> BTreeMap<String, String> {
    enc.pins().into_iter().collect()
}

/// A new fingerprint as `new` makes it: raea's base axes, or the axes of the one it --extends, with --who and --about
pub(crate) fn shaped(short: &str, args: &[String], parent: Option<Fingerprint>) -> Fingerprint {
    let mut fp = base(short);
    if let Some(p) = parent {
        fp.axes = p.axes;
        fp.extends = Some(p.name);
    }
    fp.who = flag(args, "--who").map(str::to_string).or_else(|| std::env::var("USER").ok());
    fp.about = flag(args, "--about").map(str::trim).filter(|a| !a.is_empty()).map(str::to_string);
    fp
}

fn new(enc: &mut Encoders, args: &[String]) -> Result<(String, i32)> {
    let n = *words(args).map_err(|e| anyhow!(e))?.first().ok_or_else(|| anyhow!("name it: raea fingerprint new NAME"))?;
    let dir = Folder::named(n);
    let short = dir.dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| n.into());
    let parent = match flag(args, "--extends") {
        Some(p) => Some(Folder::named(p).read(None).with_context(|| format!("--extends {p}"))?),
        None => None,
    };
    let from = parent.as_ref().map(|p| format!("{}'s axes", p.name)).unwrap_or_else(|| "raea's base taxonomy".into());
    let mut fp = shaped(&short, args, parent);
    fp.encoders = pins(enc);
    dir.create(&fp)?;
    let w = Words::build(enc, &fp)?;
    dir.write_words(&w)?;
    retrain(&dir, &fp, &w, &Ledger::default(), None)?;
    if !root().join("default").exists() && dir.dir.parent() == Some(root().as_path()) {
        std::fs::write(root().join("default"), &short)?;
    }
    Ok((
        format!(
            "made {}: {} axes from {from}, {} scoring from their words today (advice until they earn a gate)\n  edit {} to add, remove or reword axes\n  next: raea fingerprint bootstrap FILE... with your own finished work trains the mechanical axes",
            dir.dir.display(),
            fp.axes.len(),
            w.axes.len(),
            dir.ts().display()
        ),
        0,
    ))
}

/// The fingerprint, its encoders checked against this machine's, and its words, made again when they changed
fn open(enc: &mut Encoders, args: &[String], eval: Eval) -> Result<(Folder, Fingerprint, Words)> {
    let dir = Folder::named(&name(args));
    let mut fp = dir.read(Some(eval))?;
    let here = pins(enc);
    if fp.encoders.is_empty() {
        fp.encoders = here;
        dir.rewrite(&fp)?;
    } else if fp.encoders != here {
        bail!("{} was trained on other encoders ({:?}) than this machine has ({:?}); its heads refuse these columns", dir.ts().display(), fp.encoders, here);
    }
    let w = match dir.words().filter(|w| !w.stale(&fp)) {
        Some(w) => w,
        None => {
            let w = Words::build(enc, &fp)?;
            dir.write_words(&w)?;
            w
        }
    };
    Ok((dir, fp, w))
}

fn boot(enc: &mut Encoders, args: &[String], cwd: &Path, eval: Eval, say: &mut dyn FnMut(String)) -> Result<(String, i32)> {
    let (dir, fp, w) = open(enc, args, eval)?;
    let mut files: Vec<PathBuf> = words(args).map_err(|e| anyhow!(e))?.iter().map(|f| at(cwd, f)).collect();
    if let Some(list) = flag(args, "--from") {
        let list = at(cwd, list);
        let text = std::fs::read_to_string(&list).with_context(|| list.display().to_string())?;
        files.extend(text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| at(list.parent().unwrap_or(cwd), l)));
    }
    if files.is_empty() {
        bail!("give it real finished work: raea fingerprint bootstrap FILE... or --from LIST (no placeholder footage)");
    }
    let per: usize = flag(args, "--per").map_or(Ok(10), str::parse).context("--per takes a count")?;
    let mut l = dir.ledger()?;
    let who = fp.who.clone().unwrap_or_else(|| "me".into());
    let made = bootstrap::rows(enc, &who, &files, per, &mut l, say)?;
    dir.write_ledger(&l)?;
    say(format!("{} windows from {} sources; measuring held out by source", made.windows, made.sources));
    let curves = bootstrap::curves(&fp, &l);
    let ws = bootstrap::word_scores(&l, &w);
    let b = json!({"made": made, "per": per, "sources": files, "curves": curves, "words": ws});
    retrain(&dir, &fp, &w, &l, Some(b))?;
    Ok((report_text(&dir.report().unwrap_or_default()), 0))
}

/// The project a file belongs to: the folder holding .raea/project above it, else the folder it's in
fn project_of(file: &Path) -> String {
    let mut d = file.parent();
    while let Some(p) = d {
        if p.join(".raea/project").exists() {
            return p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        }
        d = p.parent();
    }
    file.parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn teach(enc: &mut Encoders, args: &[String], cwd: &Path, eval: Eval) -> Result<(String, i32)> {
    let (dir, fp, w) = open(enc, args, eval)?;
    let file = at(cwd, words(args).map_err(|e| anyhow!(e))?.first().ok_or_else(|| anyhow!("which file?"))?);
    let axis = flag(args, "--axis").ok_or_else(|| anyhow!("--axis A (or \"*\" for every axis)"))?;
    if axis != "*" && fp.axis(axis).is_none() {
        bail!("no axis {axis:?} in {}: its axes are {}", dir.ts().display(), fp.axes.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "));
    }
    let t: f64 = flag(args, "--at").ok_or_else(|| anyhow!("--at SECONDS"))?.parse().context("--at takes seconds")?;
    let span: f64 = flag(args, "--span").map_or(Ok(2.0), str::parse).context("--span takes seconds")?;
    let region = flag(args, "--box").map(|b| -> Result<[f32; 4]> {
        let v: Vec<f32> = b.split(',').map(str::parse).collect::<Result<_, _>>().context("--box takes x0,y0,x1,y1 in 0-1")?;
        v.try_into().map_err(|_| anyhow!("--box takes four numbers"))
    });
    let c = enc.columns(&file)?;
    let a = (t - span / 2.0).clamp(0.0, (c.dur - span).max(0.0));
    let b = (a + span).min(c.dur);
    let bad = !has(args, "--good");
    let row = Row {
        media: c.media.clone(),
        a,
        b,
        axis: axis.into(),
        bad,
        weight: 1.0,
        who: fp.who.clone().unwrap_or_else(|| "me".into()),
        source: "teach".into(),
        project: flag(args, "--project").map_or_else(|| project_of(&file), str::to_string),
        kind: None,
        region: region.transpose()?,
        words: flag(args, "--words").map(str::to_string),
        at: 0,
    };
    let mut l = dir.ledger()?;
    l.push(row, &features(&c, a, b));
    dir.write_ledger(&l)?;
    let tr = retrain(&dir, &fp, &w, &l, None)?;
    let h = tr.heads.iter().find(|h| h.axis == axis);
    let (p, n) = l.for_axis(axis).iter().fold((0, 0), |(p, n), &i| if l.rows[i].bad { (p + 1, n) } else { (p, n + 1) });
    Ok((
        format!(
            "taught {axis}: {} at {} in {}\n  {axis} has {p} defect and {n} clean labels; its head: {}",
            if bad { "the defect" } else { "clean" },
            clock(t),
            file.display(),
            h.map_or("every axis's clean labels grew".to_string(), |h| format!("{} ({})", h.state, h.why))
        ),
        0,
    ))
}

/// raea check's findings on the file, in seconds: (rule, from, to, what)
fn checks(file: &Path) -> Vec<(String, f64, f64, String)> {
    let Ok(found) = raea_app_check::floor(&[file.display().to_string()], &[], &[]) else { return Vec::new() };
    found.into_iter().map(|f| {
        let (a, b) = f.frames.map_or((0.0, 0.0), |[a, b]| (a as f64 / 30.0, b as f64 / 30.0));
        (f.check, a, b, f.says)
    }).collect()
}

/// A check wins over a head (FP4c): a moment on an axis a check measures is no gate unless the check found it too
fn arbitrate(s: &mut Scored, fp: &Fingerprint, found: &[(String, f64, f64, String)]) {
    for m in &mut s.moments {
        let Some(rule) = fp.axis(&m.axis).and_then(|a| a.check.as_deref()) else { continue };
        let agrees = found.iter().any(|(r, a, b, _)| r == rule && *a <= m.b + 1.0 && *b >= m.a - 1.0);
        if !agrees {
            m.gating = false;
            m.state = format!("{}; the {rule} check found none", m.state);
        }
    }
}

fn scored(enc: &mut Encoders, args: &[String], cwd: &Path, eval: Eval) -> Result<(String, i32)> {
    let (dir, fp, w) = open(enc, args, eval)?;
    let file = at(cwd, words(args).map_err(|e| anyhow!(e))?.first().ok_or_else(|| anyhow!("which file?"))?);
    let c = enc.columns(&file)?;
    let trained = dir.trained()?.map(|(t, _)| t);
    let mut s = score(&fp, Some(&w), trained.as_ref(), &c, &file.display().to_string());
    let found = checks(&file);
    arbitrate(&mut s, &fp, &found);
    let mut out = if has(args, "--json") {
        serde_json::to_string_pretty(&json!({"scored": s, "checks": found.iter().map(|(r, a, b, says)| json!({"check": r, "a": a, "b": b, "says": says})).collect::<Vec<_>>()}))?
    } else {
        let mut t = summary(&s);
        t += &format!("checks: {}\n", if found.is_empty() { "no findings".to_string() } else { format!("{} findings", found.len()) });
        for (r, a, _, says) in &found {
            t += &format!("  {} {r}: {says}\n", clock(*a));
        }
        t
    };
    if has(args, "--timeline") {
        out += &timeline(&s);
    }
    if let Some(png) = flag(args, "--sheet") {
        let png = at(cwd, png);
        let frames: Vec<media::Rgb> = (0..12).filter_map(|i| media::frames(&file, FPS, c.dur * (i as f64 + 0.5) / 12.0, 0.25).ok()?.into_iter().next()).collect();
        let pcm = media::sound(&file, 16_000)?;
        let ticks: Vec<Tick> = found.iter().map(|(r, a, b, _)| Tick { a: *a, b: *b, what: r.clone() }).collect();
        sheet(&s, &frames, &pcm, &ticks, &|axis| fp.axis(axis).map(|a| a.modality), &png)?;
        out += &format!("sheet: {}\n", png.display());
    }
    Ok((out, i32::from(s.moments.iter().any(|m| m.gating))))
}

/// `raea fingerprint report`: report.json, read; no encoders needed
pub(crate) fn report_here(args: &[String]) -> i32 {
    let dir = Folder::named(&name(args));
    match dir.report() {
        Some(r) if has(args, "--json") => {
            println!("{}", serde_json::to_string_pretty(&r).unwrap_or_default());
            0
        }
        Some(r) => {
            println!("{}", report_text(&r).trim_end());
            0
        }
        None => {
            eprintln!("raea fingerprint report: no report at {} (`raea fingerprint new NAME` makes one)", dir.dir.display());
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::shaped;
    use raea_sys_fingerprint::taxonomy::base;

    fn args(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    /// FP14a: a new fingerprint says what it's for, and one that extends another starts from its axes
    #[test]
    fn a_new_fingerprint_says_what_its_for_and_what_it_builds_on() {
        let own = shaped("me", &args(&["me", "--who", "w"]), None);
        assert_eq!((own.about.as_deref(), own.extends.as_deref(), own.who.as_deref()), (None, None, Some("w")));
        assert_eq!(own.axes, base("me").axes);

        let mut parent = base("me");
        parent.axes.truncate(2);
        parent.axes[0].words = vec!["a soft picture".into()];
        let ads = shaped("ads", &args(&["ads", "--about", " Short ads for one brand ", "--extends", "me"]), Some(parent.clone()));
        assert_eq!(ads.name, "ads");
        assert_eq!(ads.about.as_deref(), Some("Short ads for one brand"));
        assert_eq!(ads.extends.as_deref(), Some("me"));
        assert_eq!(ads.axes, parent.axes);
        assert_eq!(shaped("x", &args(&["x", "--about", "  "]), None).about, None);
    }
}
