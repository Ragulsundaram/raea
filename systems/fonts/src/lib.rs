//! raea's bundled faces (core/fonts) in one parley font context, sans-serif mapped onto Inter as the Chrome oracle
//! has it (docs/blitz-bevy.feature B0d). It's made once a process for each folder of faces, and every page's document
//! and bevy_text's FontCx (`share_with_text`) are given clones of it: the faces are read, decoded and registered once.

/// Google Fonts families a page names, fetched once into raea's cache
pub mod google;
/// fontique's own generic families under names of their own, for bevy_ui's generic FontSources
pub mod bevy_generics;
/// bevy_text's glyph atlases for a text, as Bevy's FontAtlasSet holds them (the fonts.atlases native call)
pub mod atlas;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use blitz_dom::FontContext;
use parley::fontique::{Blob, FamilyId, FontInfoOverride, FontStyle, FontWeight, GenericFamily};
use serde_json::Value;

/// A font context with the faces in `dir` (WOFF2, named by its index.json) and the system's: a clone of the one made
/// for `dir` the first time it's asked for.
pub fn context(dir: &Path) -> FontContext {
    made(dir).0
}

/// `context(dir)` with fontique's own generic families (sans-serif, serif and monospace as the system has them), for
/// bevy_text: the same faces, and Bevy's text drawn as before.
pub fn text_context(dir: &Path) -> FontContext {
    let (mut cx, own) = made(dir);
    for (generic, ids) in own {
        cx.collection.set_generic_families(generic, ids.into_iter());
    }
    cx
}

/// The App's bevy_text FontCx made from `text_context(dir)`, when it has one and it isn't already (bevy_text registers
/// its font assets again on top of it).
pub fn share_with_text(world: &mut bevy::ecs::world::World, dir: &Path) {
    if world.get_resource::<bevy::text::FontCx>().is_some_and(|cx| !cx.has_base()) {
        world.resource_mut::<bevy::text::FontCx>().set_base(text_context(dir));
    }
}

/// fontique's own generic families, before raea maps them as Chrome does
type Own = Vec<(GenericFamily, Vec<FamilyId>)>;

/// The context for `dir` and its own generic families, made once
fn made(dir: &Path) -> (FontContext, Own) {
    static MADE: OnceLock<Mutex<HashMap<PathBuf, (FontContext, Own)>>> = OnceLock::new();
    let mut made = MADE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    made.entry(dir.to_path_buf()).or_insert_with(|| build(dir)).clone()
}

fn build(dir: &Path) -> (FontContext, Own) {
    let mut cx = FontContext::new();
    let own: Own = [GenericFamily::SansSerif, GenericFamily::Serif, GenericFamily::Monospace]
        .into_iter()
        .map(|g| (g, cx.collection.generic_families(g).collect()))
        .collect();
    bevy_generics::register(&mut cx);
    let index: HashMap<String, (String, f32, bool)> = std::fs::read(dir.join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<Value>>(&b).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|f| {
            let file = f.get("file")?.as_str()?.to_string();
            let family = f.get("family")?.as_str()?.replace('-', " ");
            let weight = f.get("weight")?.as_str()?.parse().ok()?;
            Some((file, (family, weight, f.get("style").and_then(Value::as_str) == Some("italic"))))
        })
        .collect();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|x| x.to_str()) == Some("woff2")).collect())
        .unwrap_or_default();
    files.sort();
    for p in files {
        let Some(ttf) = std::fs::read(&p).ok().and_then(|b| wuff::decompress_woff2(&b).ok()) else {
            bevy::log::warn!("raea-page: {} doesn't decode", p.display());
            continue;
        };
        let named = p.file_name().and_then(|f| index.get(f.to_str()?));
        // SVG text and a canvas's text (usvg's font database) have the bundled faces too, by the same family name: a
        // composition's canvas measured and drew Inter in a fallback face (layout-reads LR6)
        blitz_dom::util::add_svg_font(ttf.clone(), named.map(|(family, _, _)| family.as_str()));
        let info = named.map(|(family, weight, italic)| FontInfoOverride {
            family_name: Some(family.as_str()),
            weight: Some(FontWeight::new(*weight)),
            style: italic.then_some(FontStyle::Italic),
            ..Default::default()
        });
        cx.collection.register_fonts(Blob::from(ttf), info);
    }
    let family = |cx: &mut FontContext, name: &str| cx.collection.family_id(name);
    if let Some(inter) = family(&mut cx, "Inter") {
        cx.collection.set_generic_families(GenericFamily::SansSerif, [inter].into_iter());
    }
    // monospace is Menlo, as Chrome on macOS has it
    #[cfg(target_vendor = "apple")]
    if let Some(menlo) = family(&mut cx, "Menlo") {
        cx.collection.set_generic_families(GenericFamily::Monospace, [menlo].into_iter());
    }
    // symbols no face in the list has (✎ ▢ ✦ ★ ⌫ ...): CoreText's choices, as Chrome takes them, in the order that
    // gives CoreText's pick for each (checked against CTFontCreateForString over 30 symbols: all but ⚡, an emoji)
    #[cfg(target_vendor = "apple")]
    {
        let symbols: Vec<_> = ["Lucida Grande", "Hiragino Sans", "Zapf Dingbats", "Apple Symbols", "Apple Color Emoji"].iter().filter_map(|n| family(&mut cx, n)).collect();
        for script in [parley::fontique::Script::COMMON, parley::fontique::Script::from_bytes(*b"Latn")] {
            let mut list: Vec<_> = cx.collection.fallback_families(script).collect();
            list.extend(symbols.iter().filter(|f| !list.contains(f)).copied().collect::<Vec<_>>());
            cx.collection.set_fallbacks(script, list.into_iter());
        }
    }
    #[cfg(target_vendor = "apple")]
    for name in ["Helvetica", "Times", "Courier"] {
        let path = format!("/System/Library/Fonts/{name}.ttc");
        match std::fs::read(&path) {
            Ok(mut b) => {
                boost(&mut b);
                cx.collection.register_fonts(Blob::from(b), None);
            }
            Err(e) => bevy::log::warn!("raea-page: {path}: {e}"),
        }
    }
    // serif, and so the standard face a list with no installed face falls to, is Times, as Chrome on macOS has it
    #[cfg(target_vendor = "apple")]
    if let Some(times) = family(&mut cx, "Times") {
        cx.collection.set_generic_families(GenericFamily::Serif, [times].into_iter());
    }
    (cx, own)
}

/// parley's MissingHook: a character no face has (the bundled ones, the system's) is said once a process, rather than
/// drawn as tofu without a word. Spaces, controls, joiners and variation selectors aren't glyphs anyone sees.
pub fn missing(text: &str) {
    static SAID: std::sync::Mutex<Vec<char>> = std::sync::Mutex::new(Vec::new());
    let Ok(mut said) = SAID.lock() else { return };
    for c in text.chars() {
        if c.is_whitespace() || c.is_control() || matches!(c, '\u{200b}'..='\u{200f}' | '\u{fe00}'..='\u{fe0f}') || said.contains(&c) {
            continue;
        }
        said.push(c);
        // the bundled Noto Sans JP is a Latin subset: kanji and kana come from the system's faces (fonts-noto-cjk on
        // Linux), as on a Mac
        let cjk = matches!(c, '\u{3000}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '\u{ac00}'..='\u{d7af}' | '\u{ff00}'..='\u{ffef}');
        let hint = if cjk && cfg!(not(target_vendor = "apple")) { " (install a CJK font, e.g. fonts-noto-cjk)" } else { "" };
        bevy::log::warn!("raea-page: no font has {c:?} (U+{:04X}), so it draws as a missing glyph{hint}", c as u32);
    }
}

/// Chrome on macOS (as Safari) gives Times, Helvetica and Courier a taller ascent, to match their Microsoft
/// counterparts: their ascent and descent rounded to whole pixels, then 15% of the sum added to the ascent
/// (FontMetrics::AscentDescentWithHacks; raea-runtime-js's css.rs `extra`). That is per size in pixels, which no ascent in
/// the font's units gives at every size, so the system's faces are registered again marked for it, with the OS/2
/// vendor `RaeA`, and the vendored parley_engine applies the hack to marked faces as it measures them: `line-height:
/// normal` and where text sits in its line are Chrome's. `font` is a TrueType collection or a single face.
#[cfg(target_vendor = "apple")]
fn boost(font: &mut [u8]) {
    let u16_at = |b: &[u8], o: usize| u16::from_be_bytes([b[o], b[o + 1]]);
    let u32_at = |b: &[u8], o: usize| u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
    let faces: Vec<usize> = if font.starts_with(b"ttcf") { (0..u32_at(font, 8)).map(|i| u32_at(font, 12 + 4 * i)).collect() } else { vec![0] };
    for face in faces {
        let n = u16_at(font, face + 4) as usize;
        let os2 = (0..n).map(|i| face + 12 + 16 * i).find(|&r| &font[r..r + 4] == b"OS/2").map(|r| u32_at(font, r + 8));
        if let Some(os2) = os2 {
            font[os2 + 58..os2 + 62].copy_from_slice(b"RaeA");
        }
    }
}

/// CoreText's pick for `text` after the face `face` (a collection's `index`-th face, `blob_id` its cache key), the way
/// Chrome on macOS falls back (FontCache::PlatformFallbackFontForCharacter, CTFontCreateForString): its family and its
/// face's CSS weight (OS/2 usWeightClass) and italic. CoreText's cascade depends on the face it starts from (a
/// monospaced face's missing symbols come from Menlo, Helvetica Bold's ▌ from Hiragino Sans W6, ✓ after it from
/// Lucida Grande), which a fallback list per script can't give. None when CoreText keeps the face, has nothing
/// (LastResort) or picks a hidden system face (a `.`-named one the collection doesn't list). Each face is made once and
/// each pick asked once.
#[cfg(target_vendor = "apple")]
pub fn coretext_fallback(face: &[u8], blob_id: u64, index: u32, text: &str) -> Option<(String, u16, bool)> {
    use objc2_core_foundation::{CFRange, CFString};
    use objc2_core_text::{CTFontSymbolicTraits, CTFontTableOptions, kCTFontTableOS2};
    use std::cell::RefCell;
    type Pick = Option<(String, u16, bool)>;
    thread_local! {
        static PICKS: RefCell<HashMap<(u64, u32), HashMap<String, Pick>>> = RefCell::new(HashMap::new());
    }
    // looked up by the text as it comes, so a frame of known picks allocates nothing (ZeroHeap)
    if let Some(p) = PICKS.with_borrow(|m| m.get(&(blob_id, index)).and_then(|m| m.get(text)).cloned()) {
        return p;
    }
    let pick = coretext_face(face, blob_id, index).and_then(|base| unsafe {
        let s = CFString::from_str(text);
        let f = base.for_string(&s, CFRange { location: 0, length: text.encode_utf16().count() as isize });
        let family = f.family_name().to_string();
        if family == base.family_name().to_string() || family.starts_with('.') {
            return None;
        }
        let weight = f.table(kCTFontTableOS2, CTFontTableOptions::empty()).map(|t| t.to_vec()).filter(|t| t.len() >= 6).map_or(400, |t| u16::from_be_bytes([t[4], t[5]]));
        let italic = f.symbolic_traits().contains(CTFontSymbolicTraits::TraitItalic);
        Some((family, weight, italic))
    });
    PICKS.with_borrow_mut(|m| m.entry((blob_id, index)).or_default().insert(text.to_string(), pick.clone()));
    pick
}

/// The CoreText font of a face's data, made once a face (None when CoreText can't read it)
#[cfg(target_vendor = "apple")]
fn coretext_face(face: &[u8], blob_id: u64, index: u32) -> Option<objc2_core_foundation::CFRetained<objc2_core_text::CTFont>> {
    use objc2_core_foundation::{CFArray, CFData, CFRetained};
    use objc2_core_text::{CTFont, CTFontDescriptor, CTFontManagerCreateFontDescriptorsFromData};
    use std::cell::RefCell;
    thread_local! {
        static FACES: RefCell<HashMap<(u64, u32), Option<CFRetained<CTFont>>>> = RefCell::new(HashMap::new());
    }
    FACES.with_borrow_mut(|m| {
        m.entry((blob_id, index))
            .or_insert_with(|| unsafe {
                let descs = CTFontManagerCreateFontDescriptorsFromData(&CFData::from_bytes(face));
                let descs: CFRetained<CFArray<CTFontDescriptor>> = CFRetained::cast_unchecked(descs);
                let d = descs.get(index as usize)?;
                Some(CTFont::with_font_descriptor(&d, 16.0, std::ptr::null()))
            })
            .clone()
    })
}

/// Where CoreGraphics draws the tops of a face's `o` and `H` (`glyphs`) at `ppem`, in px above the baseline:
/// vello_encoding's GlyphTops hook, set unless RAEA_PAGE_CG_YMAP=0. Chrome on macOS draws text through CoreGraphics,
/// which moves a TrueType face's glyphs in y at whole sizes of 18 px and under (the glyph's outline at fractional sizes
/// and from 20 px): Helvetica's H 11.48 px tall at 16 px is drawn 12 rows tall, the o's top 8.61 at 8.98, by a rule per
/// face no hinter reproduces (skrifa's interpreter and autohinter both round some faces' heights the other way). So
/// it's asked: each glyph is drawn by CoreGraphics and fitted with the y scale at which CoreGraphics' own fill of its
/// unhinted path (CTFontCreatePathForGlyph) matches it row by row; the top is the path's, at that scale. Glyphs drawn
/// through the two (vello_encoding's YMap) come within 0.03-0.08 of CoreGraphics' rows (a row profile's L1 distance
/// over its ink, Times, Helvetica, Arial, Georgia, Verdana, Menlo and Fira Mono at 12-18 px), against 0.04-0.21
/// unhinted. None past 18 px, at a fractional size, or when CoreGraphics can't read the face.
#[cfg(target_vendor = "apple")]
pub fn coregraphics_tops(blob_id: u64, face: &[u8], index: u32, ppem: f32, location: &[(u32, f32)], glyphs: [u32; 2]) -> Option<[f32; 2]> {
    use objc2_core_foundation::CFNumber;
    if !(6.0..=18.0).contains(&ppem) || (ppem - ppem.round()).abs() > 1e-3 {
        return None;
    }
    let base = coretext_face(face, blob_id, index)?;
    // a variable face at its location: CoreGraphics moves those glyphs too (Geist's x-height at 13 px)
    let desc = location.iter().fold(unsafe { base.font_descriptor() }, |d, &(tag, value)| unsafe {
        d.copy_with_variation(&CFNumber::new_i64(tag as i64), value as f64)
    });
    let font = unsafe { base.copy_with_attributes(ppem as f64, std::ptr::null(), Some(&desc)) };
    let mut tops = [0.0; 2];
    for (top, glyph) in tops.iter_mut().zip(glyphs) {
        *top = coregraphics_top(&font, u16::try_from(glyph).ok()?, ppem)?;
    }
    Some(tops)
}

/// One glyph's top as CoreGraphics draws it (coregraphics_tops)
#[cfg(target_vendor = "apple")]
fn coregraphics_top(font: &objc2_core_text::CTFont, glyph: u16, ppem: f32) -> Option<f32> {
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::{CGBitmapContextCreate, CGColorSpace, CGContext, CGPath};
    use std::ptr::NonNull;
    // a square three sizes across, the baseline a third up, the glyph 4 px in: the same place in every drawing
    let n = ppem.ceil() as usize * 3 + 8;
    let base = (n / 3) as f64;
    let space = CGColorSpace::new_device_gray()?;
    // each row's coverage, top row first, of what `draw` fills in white on black, antialiased as Skia has
    // CoreGraphics draw text on macOS (no font smoothing, subpixel positioning, no subpixel quantization)
    let rows = |draw: &dyn Fn(&CGContext)| -> Option<Vec<f32>> {
        let mut px = vec![0u8; n * n];
        let c = unsafe { CGBitmapContextCreate(px.as_mut_ptr().cast(), n, n, 8, n, Some(&space), 0)? };
        let c = Some(&*c);
        CGContext::set_should_antialias(c, true);
        CGContext::set_allows_font_smoothing(c, false);
        CGContext::set_should_smooth_fonts(c, false);
        CGContext::set_allows_font_subpixel_positioning(c, true);
        CGContext::set_should_subpixel_position_fonts(c, true);
        CGContext::set_allows_font_subpixel_quantization(c, false);
        CGContext::set_should_subpixel_quantize_fonts(c, false);
        CGContext::set_gray_fill_color(c, 1.0, 1.0);
        draw(c.unwrap());
        Some(px.chunks(n).map(|row| row.iter().map(|&v| v as f32 / 255.0).sum()).collect())
    };
    let at = CGPoint { x: 4.0, y: base };
    let want = rows(&|c| unsafe { font.draw_glyphs(NonNull::from(&glyph), NonNull::from(&at), 1, c) })?;
    let path = unsafe { font.path_for_glyph(glyph, std::ptr::null()) }?;
    let off = |ys: f64| -> Option<f32> {
        let got = rows(&|c| {
            CGContext::translate_ctm(Some(c), 4.0, base);
            CGContext::scale_ctm(Some(c), 1.0, ys);
            CGContext::add_path(Some(c), Some(&path));
            CGContext::fill_path(Some(c));
        })?;
        Some(got.iter().zip(&want).map(|(a, b)| (a - b).abs()).sum())
    };
    // the y scale: every 0.01 from 0.85 to 1.15, then every 0.0005 around the best
    let best = |from: f64, step: f64, count: usize| -> Option<f64> {
        let mut best = (f32::MAX, from);
        for i in 0..count {
            let ys = from + step * i as f64;
            let d = off(ys)?;
            if d < best.0 {
                best = (d, ys);
            }
        }
        Some(best.1)
    };
    let coarse = best(0.85, 0.01, 31)?;
    let ys = best(coarse - 0.01, 0.0005, 41)?;
    let CGRect { origin, size: CGSize { height, .. } } = CGPath::bounding_box(Some(&path));
    Some(((origin.y + height) * ys) as f32)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn shape(cx: &mut blitz_dom::FontContext, text: &str) {
        let mut lcx = parley::LayoutContext::<[u8; 4]>::new();
        let mut b = lcx.ranged_builder(cx, text, 1.0, true);
        b.push_default(parley::StyleProperty::FontFamily(parley::FontFamily::Source("Inter".into())));
        b.build(text);
    }

    /// parley tells raea of a character no face has (a private-use one), after every fallback, and of nothing else
    #[test]
    fn a_character_no_face_has_is_told() {
        parley::set_missing_hook(|t| SEEN.lock().unwrap().push(t.to_string()));
        let mut cx = super::context(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../core/fonts"));
        shape(&mut cx, "Quick fox, é ñ");
        assert!(SEEN.lock().unwrap().is_empty(), "{:?}", SEEN.lock().unwrap());
        shape(&mut cx, "a\u{e123}b");
        assert_eq!(SEEN.lock().unwrap().as_slice(), ["\u{e123}"]);
    }
}
