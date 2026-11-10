//! A composition's frames read into worlds of facts (docs/direction.feature DR1). React is stepped to each frame as
//! `raea scene lint` steps it, and nothing is drawn: the frame's 3D items (scene.rs), its page laid out with no paint
//! (page.rs) and its timeline's shots, clips, cues and lines (timeline.rs) become entities with components
//! (raea-core-facts), so one query can span every medium a piece has.

mod covers;
mod page;
mod pixels;
mod renders;
mod scene;
mod source;
mod timeline;

pub use renders::Render;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bevy::prelude::{App, MinimalPlugins};
pub use raea_core_facts::*;
// where a render's manifest is (every render writes one beside its output, DR2)
pub use raea_sys_layers::manifest_for;
use raea_runtime_bundle::{bundle_composition, CompOpts};
use raea_core_scene::num;
use serde_json::Value;

type R<T> = Result<T, String>;

/// What a read is asked for: the frames, and whether the page is laid out (without it, its text carries
/// `needs: "render"`, as no box or size is known).
pub struct Ask {
    /// none: every frame of the composition (`raea check`'s checks that range over all of them)
    pub frames: Vec<i64>,
    pub layout: bool,
    /// which render's facts join the world (DR2)
    pub render: Render,
    /// the frames it joins (None: every frame asked); `raea check` joins it only where a check needs a render, as a
    /// text's contrast is measured on each frame it joins
    pub join_on: Option<BTreeSet<i64>>,
}

/// One stepped frame: its 3D, its React tree, and its page as laid out.
struct Snap {
    scene: Vec<raea_core_scene::Item>,
    tree: Vec<timeline::El>,
    laid: Option<Value>,
}

/// The frames of `tsx`'s composition (`o` picks it) as worlds, one a frame, in `ask.frames`' order. The same frame
/// gives the same world each time: frames are pure.
pub fn read(tsx: &Path, o: &CompOpts, ask: &Ask) -> R<Vec<Facts>> {
    let tsx = tsx.canonicalize().map_err(|e| format!("{}: {e}", tsx.display()))?;
    let js =std::env::temp_dir().join(format!("raea-world-{}.js", std::process::id()));
    // every element and part stamped with its place in the code, so each entity has its Source (as the stage's
    // picks have their keys, engine/page/stage); RAEA_SOURCE_STAMPS=0 in the environment keeps them off
    if std::env::var_os("RAEA_SOURCE_STAMPS").is_none() {
        // SAFETY: before any thread of the read starts (the bundler and React's runtime start below)
        unsafe { std::env::set_var("RAEA_SOURCE_STAMPS", "1") };
    }
    bundle_composition(&tsx, &js, o, None)?;
    let got = stepped(&js, &tsx, ask, o.words.as_ref());
    let _ = std::fs::remove_file(&js);
    got
}

fn stepped(js: &Path, tsx: &Path, ask: &Ask, words: Option<&Value>) -> R<Vec<Facts>> {
    use raea_runtime_react::scene::{edit_dir, scene_of, settle};
    let dir = tsx.parent().ok_or("the edit has no folder")?;
    edit_dir(dir);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(raea_runtime_react::ReactPlugin::new(js.to_path_buf()));
    app.insert_resource(raea_runtime_react::OneBatch::default());
    let conf = settle(&mut app, Some(-1), 20.0)?.ok_or("the composition's settings didn't arrive")?;
    let (w, h) = (num(&conf, "width").unwrap_or(1080.0) as u32, num(&conf, "height").unwrap_or(1920.0) as u32);
    let fps = num(&conf, "fps").unwrap_or(30.0);
    let length = num(&conf, "frames").unwrap_or(60.0) as i64;
    if let Some(f) = ask.frames.iter().find(|f| **f < 0 || **f >= length) {
        return Err(format!("there's no frame {f}: the composition has frames 0 to {}", length - 1));
    }
    let asked: Vec<i64> = if ask.frames.is_empty() { (0..length).collect() } else { ask.frames.clone() };
    let wanted: BTreeSet<i64> = asked.iter().copied().collect();
    // and each one's next frame, for what moves (the camera's velocity)
    let all: BTreeSet<i64> = wanted.iter().flat_map(|&f| [f, f + 1]).filter(|f| *f < length).collect();
    let mut page = page::Page::new(ask.layout, w, h, dir);
    let mut snaps: BTreeMap<i64, Snap> = BTreeMap::new();
    for (a, b) in raea_runtime_bake::runs(&all) {
        raea_runtime_bake::step_frames(&mut app, a, b, |app, f| {
            let world = app.world_mut();
            let scene = raea_runtime_scene::items_of(&scene_of(world));
            let (tree, laid) = if wanted.contains(&f) { (timeline::tree(world), page.laid_out(world, f, fps)) } else { (Vec::new(), None) };
            snaps.insert(f, Snap { scene, tree, laid });
            Ok(())
        })?;
    }
    let lines = timeline::project_lines(words, dir, fps);
    let mut render = renders::find(&ask.render, tsx, (w, h, fps));
    let worlds = asked
        .iter()
        .map(|&f| {
            let now = &snaps[&f];
            let mut facts = Facts::new(f, w, h, fps);
            let mut order = 0u32;
            scene::put(&mut facts, &now.scene, snaps.get(&(f + 1)).map(|n| &n.scene[..]), fps, dir, &mut order);
            timeline::put(&mut facts, &now.tree, now.laid.as_ref(), (w, h, fps), dir, &mut order);
            timeline::put_lines(&mut facts, &lines, &now.tree, f, fps);
            // what only a render knows, or what waits for one (DR2)
            if ask.render != Render::Off && ask.join_on.as_ref().is_none_or(|on| on.contains(&f)) {
                renders::join(&mut facts, render.as_mut(), &now.tree);
            }
            facts
        })
        .collect();
    if let Some(r) = render {
        r.keep();
    }
    Ok(worlds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbouring_frames_step_as_one_range() {
        let s: BTreeSet<i64> = [3, 4, 5, 9, 10, 20].into_iter().collect();
        assert_eq!(raea_runtime_bake::runs(&s), vec![(3, 6), (9, 11), (20, 21)]);
        assert!(raea_runtime_bake::runs(&BTreeSet::new()).is_empty());
    }
}
