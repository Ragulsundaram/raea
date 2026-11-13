//! Google Fonts by name: a family a page's styles name that no face here has is looked for on Google Fonts, and its
//! Latin faces (each weight and style it has) are fetched once into raea's cache (raea_io_net::cache, so a
//! render is the same frame for frame, and offline once cached) and given to the document as an @font-face would give
//! them. Chrome loads only what a stylesheet links; a composition here just names the family (craft-system.md phase
//! 8: the Director had to fetch and embed each family itself). Only the Latin subset: Blitz has no unicode-range, so
//! every subset of a weight under one name would be one face to it, and one without Latin could be the one picked.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blitz_dom::BaseDocument;
use blitz_dom::net::{FontFaceOverrides, Resource, ResourceLoadResponse};
use blitz_traits::net::Bytes;
use parley::fontique::FontStyle;

/// A family's faces: (TrueType bytes, weight, italic)
type Faces = Arc<Vec<(Bytes, f32, bool)>>;

/// Each family looked for in this process: its faces, none when Google Fonts has no such family or it can't be
/// fetched (looked for once a process, not every frame)
static FAMILIES: Mutex<Option<HashMap<String, Faces>>> = Mutex::new(None);

/// Every weight, upright and italic: Google's CSS API answers with the ones the family has
const WEIGHTS: &str = "100,100i,200,200i,300,300i,400,400i,500,500i,600,600i,700,700i,800,800i,900,900i";

/// Families a composition asked for by name (raea/text loadFont, the `fonts.google` call), as a stylesheet link
/// asks Chrome for them: fetched whatever RAEA_PAGE_GOOGLE_FONTS says, and only these when it's off
static WANTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// RAEA_PAGE_GOOGLE_FONTS=1: every family a page names (off by default: Chrome loads only what a page links)
fn every() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("RAEA_PAGE_GOOGLE_FONTS").as_deref() == Ok("1"));
    *ON
}

/// `family` wanted from Google Fonts (case aside), as a page that links its stylesheet wants it
pub fn want(family: &str) {
    let mut w = WANTED.lock().unwrap();
    if !w.iter().any(|f| f.eq_ignore_ascii_case(family)) {
        w.push(family.to_string());
    }
}

fn wanted(family: &str) -> bool {
    every() || WANTED.lock().unwrap().iter().any(|f| f.eq_ignore_ascii_case(family))
}

/// Whether a page looks for missing families at all: RAEA_PAGE_GOOGLE_FONTS=1, or a family asked for by name
pub fn enabled() -> bool {
    every() || !WANTED.lock().unwrap().is_empty()
}

/// Gives `doc` the faces of each family its elements name that it has none of and Google Fonts has; true when one
/// was given (its text is laid out again).
pub fn load_missing(doc: &mut BaseDocument) -> bool {
    let mut loaded = false;
    for family in doc.missing_font_families() {
        if !wanted(&family) {
            continue;
        }
        let faces = FAMILIES.lock().unwrap().get_or_insert_default().get(&family).cloned();
        let faces = faces.unwrap_or_else(|| {
            let found = Arc::new(fetch(&family));
            FAMILIES.lock().unwrap().get_or_insert_default().insert(family.clone(), found.clone());
            found
        });
        for (ttf, weight, italic) in faces.iter() {
            let overrides = FontFaceOverrides { family_name: Some(family.clone()), weight: Some(*weight), style: italic.then_some(FontStyle::Italic) };
            doc.load_resource(ResourceLoadResponse { request_id: usize::MAX, node_id: None, resolved_url: None, result: Ok(Resource::Font(ttf.clone(), overrides)) });
            loaded = true;
        }
    }
    loaded
}

/// The family's faces for a canvas and SVG text now (document.fonts.load, layout-reads LR7): a family the composition
/// asked for, fetched as load_missing would and given to usvg's font database at once, where the page would only
/// fetch it once an element names it. How many faces it has; 0 for one not asked for or not on Google Fonts.
pub fn load_now(family: &str) -> usize {
    if !wanted(family) {
        return 0;
    }
    let mut known = FAMILIES.lock().unwrap();
    let known = known.get_or_insert_default();
    if let Some(faces) = known.get(family) {
        return faces.len();
    }
    let faces = Arc::new(fetch(family));
    for (ttf, weight, italic) in faces.iter() {
        // each at the weight and style its @font-face gives it (one variable file is every weight of a family)
        blitz_dom::util::add_svg_font(ttf.to_vec(), Some(family), Some((*weight as u16, *italic)));
    }
    known.insert(family.to_string(), faces.clone());
    faces.len()
}

/// A family's Latin faces from Google Fonts, or none
fn fetch(family: &str) -> Vec<(Bytes, f32, bool)> {
    let url = format!("https://fonts.googleapis.com/css?family={}:{WEIGHTS}", family.replace(' ', "+"));
    let css = match raea_io_net::cache::read(&url).and_then(|b| String::from_utf8(b).map_err(|e| e.to_string())) {
        Ok(css) => css,
        Err(e) => {
            bevy::log::debug!("raea-page: no Google Fonts family {family:?} ({e})");
            return Vec::new();
        }
    };
    let faces: Vec<_> = css
        .split("/* latin */")
        .skip(1)
        .filter_map(|block| {
            let block = &block[..block.find('}')?];
            let src = block.split("url(").nth(1)?.split(')').next()?;
            let weight = block.split("font-weight:").nth(1)?.trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()?;
            let italic = block.split("font-style:").nth(1)?.trim_start().starts_with("italic");
            let woff2 = raea_io_net::cache::read(src).ok()?;
            let ttf = wuff::decompress_woff2(&woff2).ok()?;
            Some((Bytes::from(ttf), weight, italic))
        })
        .collect();
    bevy::log::info!("raea-page: Google Fonts {family:?}, {} faces", faces.len());
    faces
}

/// A face a composition loads by URL (`new FontFace(family, "url(…)")`, docs/layout-reads.feature LR7), fetched now
/// through raea's cache and given to a canvas's and an svg's text as `family`; woff2 is unpacked, TrueType and OpenType
/// are taken as they are. How many faces that gave (0 when it doesn't load). A URL already loaded is not fetched again.
pub fn face_now(family: &str, url: &str) -> usize {
    static LOADED: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
    let key = (family.to_string(), url.to_string());
    if LOADED.lock().unwrap().contains(&key) {
        return 1;
    }
    let Ok(bytes) = raea_io_net::cache::read(url) else { return 0 };
    let ttf = if bytes.starts_with(b"wOF2") { wuff::decompress_woff2(&bytes).ok() } else { Some(bytes) };
    let Some(ttf) = ttf else { return 0 };
    blitz_dom::util::add_svg_font(ttf, Some(family), None);
    LOADED.lock().unwrap().push(key);
    1
}
