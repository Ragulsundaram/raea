//! The layer pass (docs/review.feature R2b, R2c): which layer each pixel of a frame shows, so raea's player can select
//! any layer (a 3D part, an element, a text, an image) by where it is clicked. A layer is a stamped thing (engine/bundle
//! stamp.rs): its key is where its JSX is written, its place in a list and the uses of the edit's components it was
//! drawn in (`key_of`). Each frame gives a quarter-size mask of layer indices (mask px (x, y) is frame px (4x+2,
//! 4y+2)); `Recorder` keeps them with the table of layers and writes NAME.layers (file.rs).

mod file;
mod hide;
mod names;
mod raster;
pub mod read;
#[cfg(feature = "page")]
mod page;
#[cfg(feature = "page")]
mod scene;

pub use file::Recorder;
pub use hide::{hidden_css, layer_selector};
pub use raster::{Tri, raster};
#[cfg(feature = "page")]
pub use page::page_ids;

use std::cell::RefCell;
use std::path::PathBuf;

/// "file:line:col[#index]|use|use…": a stamped element or part's key from its props, `data-raea-loc`,
/// `#data-raea-idx` when it has one, then `|data-raea-use` when it was drawn inside the edit's components. None for an
/// unstamped one.
pub fn key_of(loc: Option<&str>, idx: Option<&str>, uses: Option<&str>) -> Option<String> {
    let loc = loc.filter(|l| !l.is_empty())?;
    let mut k = loc.to_string();
    if let Some(i) = idx.filter(|i| !i.is_empty()) {
        k.push('#');
        k.push_str(i);
    }
    if let Some(u) = uses.filter(|u| !u.is_empty()) {
        k.push('|');
        k.push_str(u);
    }
    Some(k)
}

/// The key of a stamped part from its props (data-raea-loc, data-raea-idx, which a numeric list key leaves a number,
/// and data-raea-use), as key_of gives it. None for an unstamped one.
pub fn key_from_props(props: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let text = |k: &str| props.get(k).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string));
    key_of(text("data-raea-loc").as_deref(), text("data-raea-idx").as_deref(), text("data-raea-use").as_deref())
}

/// One layer of the table: what NAME.layers' header lists for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub key: String,
    /// mesh | element | text | image | video | group
    pub kind: &'static str,
    pub name: String,
    /// the key of the nearest stamped thing it is inside
    pub parent: Option<String>,
    pub loc: String,
    pub uses: Vec<String>,
    /// its .map()'s key
    pub index: Option<String>,
    /// an image or video's source
    pub src: Option<String>,
}

impl Layer {
    /// A layer from its key (key_of's "loc[#index][|use|use…]"), split back into its stamp, place and uses.
    pub fn of(key: String, kind: &'static str, name: String, parent: Option<String>, src: Option<String>) -> Layer {
        let (loc, rest) = key.split_once('|').map_or((key.as_str(), None), |(l, r)| (l, Some(r)));
        let (loc, index) = loc.split_once('#').map_or((loc, None), |(l, i)| (l, Some(i.to_string())));
        let (loc, uses) = (loc.to_string(), rest.map(|r| r.split('|').map(str::to_string).collect()).unwrap_or_default());
        Layer { key, kind, name, parent, loc, uses, index, src }
    }
}

thread_local! {
    /// `raea render --layers`: the render's recorder (one render per thread)
    static ON: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// With `on` (`raea render --layers`), layers recorded from now on for the export `out` of `tsx`, written by `finish`
/// (path_for); off otherwise. The render.json beside the layers that names them, when on.
pub fn start(on: bool, out: &std::path::Path, tsx: &std::path::Path) -> Option<PathBuf> {
    let path = on.then(|| path_for(out, tsx));
    // a folder of frames is no video: its name stands for it
    let video = out.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let src = tsx.parent().map(PathBuf::from).unwrap_or_default();
    ON.with(|r| *r.borrow_mut() = path.clone().map(|p| Recorder::new(p, video).named_from(src)));
    path.map(|_| manifest_for(out, tsx))
}

/// The render.json beside an export `out` (OUT.render.json, or DIR/NAME.render.json for a folder of frames), which
/// every render writes unless `--manifest` puts it elsewhere (docs/direction.feature DR2).
pub fn manifest_for(out: &std::path::Path, tsx: &std::path::Path) -> PathBuf {
    path_for(out, tsx).with_extension("render.json")
}

/// Whether this render records its layers.
pub fn on() -> bool {
    ON.with(|r| r.borrow().is_some())
}

/// The recorder, when on.
pub fn with<T>(f: impl FnOnce(&mut Recorder) -> T) -> Option<T> {
    ON.with(|r| r.borrow_mut().as_mut().map(f))
}

/// NAME.layers written at `fps`, the recorder stopped: the file's name, beside render.json, for its "layers". None when
/// off or nothing was recorded.
pub fn finish(fps: f64) -> Option<String> {
    let rec = ON.with(|r| r.borrow_mut().take())?;
    match rec.write(fps) {
        Ok(name) => name,
        Err(e) => {
            eprintln!("raea: the layers weren't written: {e}");
            None
        }
    }
}

/// The NAME.layers beside an export `out`: OUT.layers for a video or still, DIR/NAME.layers for a folder of frames
/// (NAME the composition's).
pub fn path_for(out: &std::path::Path, tsx: &std::path::Path) -> PathBuf {
    match out.extension() {
        Some(_) => out.with_extension("layers"),
        None => out.join(tsx.file_stem().unwrap_or_default()).with_extension("layers"),
    }
}

#[cfg(test)]
mod tests;

/// RAEA_LAYERS_MUTANT (bench/layers/mutants.sh): the pass broken on purpose, so the oracle is seen to catch it.
/// `swap` the first two layers' keys, `shift` the 3D 2 px right, `occlusion` the 3D's farthest triangle wins over its
/// nearest, `under` the 3D drawn over the page, `bind` a skinned mesh drawn in its bind pose (systems/frame layer.rs).
pub fn mutant(name: &str) -> bool {
    std::env::var("RAEA_LAYERS_MUTANT").is_ok_and(|m| m == name)
}
