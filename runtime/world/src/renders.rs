//! What a composition's render knows, joined to each frame's world (docs/direction.feature DR2). Every render writes
//! its manifest beside its output (render.json: what each frame showed, as laid out); the world takes the newest of
//! the composition's renders in its project's record, or one it's given, and adds what only that render holds: each
//! text's contrast against its pixels, the frame's and each shot's light and movement, the mix's loudness, when each
//! cue is heard, what OCR reads in each text's box (DR28, pixels.rs), and, from a render with --layers, which layers
//! cover which (covers.rs). Each is measured on the output when first asked for and kept in the manifest's
//! "measured", so asking again reads it back. A world with no render, or a frame the render didn't draw, says what
//! waits for one (`Waits`).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use raea_core_facts::{record, Contrast, Cue, Facts, Mix, Ocr, Onset, Picture, Rendered, Shot, Source, Text, Waits};
use raea_sys_measure as measure;
use serde_json::{json, Value};

use crate::timeline::El;

/// Which render joins the world.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Render {
    /// the newest render of the composition in its project's record whose manifest is this composition's
    #[default]
    Newest,
    /// this output
    From(PathBuf),
    Off,
}

/// The components a render adds, and what each waits for when there is none.
const FROM_A_RENDER: [&str; 6] = ["Contrast", "Ocr", "Picture", "Mix", "Onset", "Covers"];

/// A render found: its output, its manifest, its record entry, whether the composition changed after it, and each
/// frame's place among the frames it drew.
pub(crate) struct Found {
    out: PathBuf,
    path: PathBuf,
    m: Value,
    record: Option<String>,
    stale: bool,
    at: HashMap<i64, usize>,
    /// whether "measured" gained anything to write back
    dirty: bool,
    env: Option<Vec<f32>>,
    masks: Option<Option<crate::covers::Masks>>,
    /// its frames, decoded once as they're asked for (a text's contrast, what OCR reads)
    pixels: crate::pixels::Pixels,
    /// the composition's folder, where its source stamps' files are
    dir: PathBuf,
}

fn secs(p: &Path) -> u64 {
    p.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// The render `how` names, if its manifest is this composition's (`size`: its width, height and frame rate).
pub(crate) fn find(how: &Render, tsx: &Path, size: (u32, u32, f64)) -> Option<Found> {
    let outs: Vec<(PathBuf, Option<String>, u64)> = match how {
        Render::Off => return None,
        // when it was made: the output's time (its manifest is written again whenever a measure is kept in it)
        Render::From(p) => vec![(p.clone(), None, secs(p))],
        Render::Newest => {
            let project = record::up(tsx.parent()?, ".raea/project")?;
            let of = record::rel(&project, tsx);
            // the record reads oldest first: this composition's renders, newest first
            let mut outs = Vec::new();
            for e in record::read(&project).into_iter().rev() {
                if let (true, Some(out)) = (e.by == "raea render" && e.of == of, e.out.as_deref()) {
                    outs.push((project.join(out), Some(e.id.clone()), e.at));
                }
            }
            outs
        }
    };
    outs.into_iter().find_map(|(out, record, made)| {
        let path = crate::manifest_for(&out, tsx);
        let m: Value = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
        // another composition of the file draws another size or rate
        let same = m["width"] == size.0 && m["height"] == size.1 && m["fps"].as_f64() == Some(size.2);
        let at = m["frames"].as_array()?.iter().enumerate().filter_map(|(k, f)| Some((f["frame"].as_i64()?, k))).collect();
        (same && out.exists()).then(|| Found { stale: secs(tsx) > made, out, path, m, record, at, dirty: false, env: None, masks: None, pixels: Default::default(), dir: tsx.parent().map(Path::to_path_buf).unwrap_or_default() })
    })
}

impl Found {
    fn video(&self) -> bool {
        self.out.extension().is_some_and(|x| matches!(x.to_ascii_lowercase().to_str(), Some("mp4" | "mov" | "wav")))
    }

    /// The picture drawn as the `k`th frame of the render: its file, and the second to read it at.
    fn picture_at(&self, k: usize) -> (PathBuf, f64) {
        match self.out.extension() {
            Some(_) if self.video() => (self.out.clone(), (k as f64 + 0.5) / self.m["fps"].as_f64().unwrap_or(30.0)),
            Some(_) => (self.out.clone(), 0.0),
            None => (self.out.join(format!("frame-{k:05}.png")), 0.0),
        }
    }

    /// A measure kept in the manifest under `name`/`key`, or `make`'s, kept.
    fn kept(&mut self, name: &str, key: &str, make: impl FnOnce(&mut Found) -> Value) -> Value {
        if let Some(v) = self.m["measured"][name].get(key) {
            return v.clone();
        }
        let v = make(self);
        self.keep_as(name, key, v.clone());
        v
    }

    fn keep_as(&mut self, name: &str, key: &str, v: Value) {
        if !self.m["measured"].is_object() {
            self.m["measured"] = json!({});
        }
        if !self.m["measured"][name].is_object() {
            self.m["measured"][name] = json!({});
        }
        self.m["measured"][name][key] = v;
        self.dirty = true;
    }

    /// What OCR reads of a text on the `k`th frame (`id`: the frame and the text), kept once read; where OCR can't
    /// run, what it waits for, and nothing kept.
    fn ocr(&mut self, k: usize, id: &str, bx: Option<[f64; 4]>, words: &str) -> Result<Option<Ocr>, String> {
        if let Some(v) = self.m["measured"]["ocr"].get(id) {
            return Ok(serde_json::from_value(v.clone()).ok());
        }
        let (file, sec) = self.picture_at(k);
        let video = self.video();
        let got = self.pixels.ocr((k, &file, sec, video), bx, words)?;
        self.keep_as("ocr", id, json!(got));
        Ok(got)
    }

    /// The `k`th frame's light and movement.
    fn picture(&mut self, k: usize) -> Option<Picture> {
        let v = if self.video() {
            // a video's frames are read in one pass, all of them, the first time any is asked for
            let all = self.kept("picture", "all", |r| json!(measure::pictures(&r.out).map(|ps| ps.iter().map(|p| json!([p.luma, p.motion])).collect::<Vec<_>>()).unwrap_or_default()));
            all.get(k).cloned()?
        } else {
            let (file, _) = self.picture_at(k);
            self.kept("picture", &k.to_string(), |_| measure::pictures(&file).ok().and_then(|p| p.first().map(|p| json!([p.luma, Value::Null]))).unwrap_or(Value::Null))
        };
        let r3 = |x: f64| (x * 1000.0).round() / 1000.0;
        Some(Picture { luma: r3(v[0].as_f64()?), motion: v[1].as_f64().map(r3), frames: None })
    }

    /// A cue's onset in the mix, for one starting at `at` seconds.
    fn onset(&mut self, at: f64) -> Onset {
        let v = self.kept("onsets", &format!("{at:.3}"), |r| {
            let env = r.env.get_or_insert_with(|| measure::envelope(&r.out).unwrap_or_default());
            json!(measure::onset(env, at))
        });
        let heard = v.as_f64();
        Onset { at: heard, late_ms: heard.map(|h| ((h - at) * 1000.0).round()) }
    }

    /// Writes "measured" back into the manifest, when it gained anything.
    pub(crate) fn keep(self) {
        if self.dirty {
            let tmp = self.path.with_extension(format!("json.{}.tmp", std::process::id()));
            if std::fs::write(&tmp, self.m.to_string()).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }
}

/// A manifest entry's layer key, as a Source's: its stamp, list key and uses.
fn key_of(t: &Value) -> Option<String> {
    let uses = t["use"].as_array().map(|u| u.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("|"));
    raea_sys_layers::key_of(t["loc"].as_str(), t["index"].as_str(), uses.as_deref())
}

/// What the render adds to `facts` (its frame's world), or what waits for one. `tree` is the frame's page in
/// document order (whose layers are drawn over whose).
pub(crate) fn join(facts: &mut Facts, found: Option<&mut Found>, tree: &[El]) {
    let Some(r) = found else {
        return facts.set(&Waits(FROM_A_RENDER.iter().map(|c| (c.to_string(), "a render".to_string())).collect()));
    };
    facts.set(&Rendered { out: r.out.display().to_string(), manifest: r.path.display().to_string(), record: r.record.clone(), stale: r.stale });
    let mut waits = BTreeMap::new();
    if r.video() {
        let mix = r.kept("mix", "all", |r| json!(measure::loudness(&r.out)));
        facts.set(&Mix { lufs: mix[0].as_f64(), peak: mix[1].as_f64() });
        for e in facts.find(&["Cue"]).iter().map(|e| e.id).collect::<Vec<_>>() {
            if let Some(c) = facts.get::<Cue>(e) {
                facts.put(e, &r.onset(c.at));
            }
        }
    } else {
        for c in ["Mix", "Onset"] {
            waits.insert(c.to_string(), "a video render".to_string());
        }
    }
    let Some(k) = r.at.get(&facts.frame).copied() else {
        for c in ["Contrast", "Picture", "Covers"] {
            waits.insert(c.to_string(), format!("a render of frame {}", facts.frame));
        }
        return facts.set(&Waits(waits));
    };
    if let Some(p) = r.picture(k) {
        facts.set(&p);
    }
    shots(facts, r);
    let laid = r.m["frames"][k].clone();
    let mut boxed = Vec::new();
    let (size, video, (file, sec)) = ((facts.width, facts.height), r.video(), r.picture_at(k));
    for e in facts.find(&["Text"]).iter().map(|e| e.id).collect::<Vec<_>>() {
        let (Some(t), key) = (facts.get::<Text>(e), facts.get::<Source>(e).and_then(|s| s.key)) else { continue };
        let id = format!("{}|{}", facts.frame, key.clone().unwrap_or_else(|| t.words.clone()));
        // what OCR reads in its box (DR28)
        match r.ocr(k, &id, t.box_, &t.words) {
            Ok(Some(o)) => facts.put(e, &o),
            Ok(None) => {}
            Err(why) => {
                waits.insert("Ocr".into(), why);
            }
        }
        // its own entry by its layer key, else the one with its words
        let texts = || laid["text"].as_array().into_iter().flatten();
        let entry = texts().find(|x| key.is_some() && key_of(x) == key).or_else(|| texts().find(|x| x["text"].as_str() == Some(t.words.as_str())));
        let (Some(x), Some(bx)) = (entry, t.box_) else { continue };
        let b: Vec<f64> = x["box"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect();
        let crop = (b.len() == 4).then(|| [b[0], b[1], b[2], b[3]].map(|v| v.max(0.0)));
        let color = x["color"].as_str().unwrap_or("").to_string();
        // measured on the frame as decoded once for every text on it (raea check's ratio, raea-sys-measure)
        let ratio = r.kept("contrast", &id, |r| json!(crop.and_then(|c| r.pixels.contrast((k, &file, sec, video), c, size, &color))));
        if let Some(ratio) = ratio.as_f64() {
            facts.put(e, &Contrast { ratio: (ratio * 10.0).round() / 10.0 });
        }
        if let Some(k) = key {
            boxed.push((e, k, bx));
        }
    }
    covers(facts, r, &laid, tree, boxed, &mut waits);
    if !waits.is_empty() {
        facts.set(&Waits(waits));
    }
}

/// Each shot's light and movement: the mean over its frames the render drew. Its first frame's change is the cut
/// into it, not movement in it, so its motion is left out.
fn shots(facts: &mut Facts, r: &mut Found) {
    for e in facts.find(&["Shot"]).iter().map(|e| e.id).collect::<Vec<_>>() {
        let Some(s) = facts.get::<Shot>(e) else { continue };
        let mut ks: Vec<(i64, usize)> = r.at.iter().filter(|(f, _)| **f >= s.from && s.to.is_none_or(|t| **f < t)).map(|(f, k)| (*f, *k)).collect();
        ks.sort();
        let ps: Vec<(i64, Picture)> = ks.into_iter().filter_map(|(f, k)| Some((f, r.picture(k)?))).collect();
        let (mut luma, mut motions) = (Vec::new(), Vec::new());
        for (f, p) in &ps {
            luma.push(p.luma);
            if let (Some(m), true) = (p.motion, *f != s.from) {
                motions.push(m);
            }
        }
        if let Some(luma) = mean(&luma) {
            facts.put(e, &Picture { luma, motion: mean(&motions), frames: Some(ps.len() as u32) });
        }
    }
}

/// The mean of `v` to three places; None for none.
fn mean(v: &[f64]) -> Option<f64> {
    let n = v.len() as f64;
    (n > 0.0).then(|| (v.iter().sum::<f64>() / n * 1000.0).round() / 1000.0)
}

/// Covers from the render's layers (a render with --layers): each boxed layer, and footage by its box this frame.
fn covers(facts: &mut Facts, r: &mut Found, laid: &Value, tree: &[El], mut boxed: Vec<(u32, String, [f64; 4])>, waits: &mut BTreeMap<String, String>) {
    if r.masks.is_none() {
        let named = r.m["layers"].as_str().map(str::to_string);
        r.masks = Some(named.and_then(|n| crate::covers::read(&r.path.with_file_name(n)).ok()));
    }
    let Some(masks) = r.masks.as_ref().and_then(Option::as_ref) else {
        waits.insert("Covers".into(), "a render with --layers".into());
        return;
    };
    let (w, h) = (facts.width as f64, facts.height as f64);
    for v in laid["videos"].as_array().into_iter().flatten() {
        let b: Vec<f64> = v["box"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect();
        let found = key_of(v).and_then(|k| Some((crate::covers::keyed(facts, &k)?, k)));
        if let (Some((e, k)), 4) = (found, b.len()) {
            boxed.push((e, k, [b[0] / w, b[1] / h, (b[0] + b[2]) / w, (b[1] + b[3]) / h]));
        }
    }
    let order: HashMap<String, usize> = tree.iter().enumerate().filter_map(|(i, el)| Some((raea_sys_layers::key_from_props(&el.props)?, i))).collect();
    // a covering layer as the world has it when it holds the element: its Source, the component it was drawn in
    let source = |k: &str| tree.iter().find(|el| raea_sys_layers::key_from_props(&el.props).as_deref() == Some(k)).and_then(|el| crate::source::of(&el.props, &r.dir));
    crate::covers::put(facts, masks, &boxed, &order, &source);
}

#[cfg(test)]
mod tests;
