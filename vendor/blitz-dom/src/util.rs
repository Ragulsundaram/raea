use crate::node::{Node, NodeData};
use color::{AlphaColor, Srgb};
use keyboard_types::Modifiers;
use std::borrow::Cow;
use style::color::AbsoluteColor;

#[cfg(target_os = "macos")]
pub(crate) const ACTION_MOD: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
pub(crate) const ACTION_MOD: Modifiers = Modifiers::CONTROL;

pub type Color = AlphaColor<Srgb>;

/// Decode raw font bytes, decompressing WOFF/WOFF2 if the `woff` feature is enabled.
/// Returns the original slice unchanged for TTF/OTF input, and also on decompression
/// failure. With the `woff` feature disabled, all input passes through unchanged.
pub fn decode_font_bytes(bytes: &[u8]) -> Cow<'_, [u8]> {
    if bytes.len() < 4 {
        return Cow::Borrowed(bytes);
    }
    match &bytes[0..4] {
        #[cfg(feature = "woff")]
        b"wOFF" => wuff::decompress_woff1(bytes)
            .map(Cow::Owned)
            .unwrap_or_else(|_| {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to decompress woff1 font");
                Cow::Borrowed(bytes)
            }),
        #[cfg(feature = "woff")]
        b"wOF2" => wuff::decompress_woff2(bytes)
            .map(Cow::Owned)
            .unwrap_or_else(|_| {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to decompress woff2 font");
                Cow::Borrowed(bytes)
            }),
        _ => Cow::Borrowed(bytes),
    }
}

#[cfg(feature = "svg")]
use std::sync::{Arc, LazyLock, Mutex};
#[cfg(feature = "svg")]
use usvg::fontdb;
#[cfg(feature = "svg")]
pub(crate) static FONT_DB: LazyLock<Mutex<Arc<fontdb::Database>>> = LazyLock::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    Mutex::new(Arc::new(db))
});

/// raea: the faces the app bundles, by family in lower case: a page's own faces of a family take its place, as an
/// @font-face shadows a local font of the same name in Chrome
#[cfg(feature = "svg")]
static BUNDLED: LazyLock<Mutex<std::collections::HashMap<String, Vec<fontdb::ID>>>> = LazyLock::new(Default::default);

/// raea: a face the page gives SVG text (its @font-face's), as HTML text has it; `family` is the name the page knows
/// it by (an @font-face's font-family), which the face is found by first. `face`: the @font-face's weight and whether
/// it's italic. SVG text finds a face by them, as Chrome does, not by the file's own: Google serves one variable file
/// as each of Inter's weights, whose own weight is 400, so a 500 found a 400 face (or the bundled static 400) and 600
/// the bundled static 700, and the text drew at 400 or 700 (layout-reads LR10). The app's bundled faces of the family
/// leave once the page gives its own.
#[cfg(feature = "svg")]
pub fn add_svg_font(bytes: Vec<u8>, family: Option<&str>, face: Option<(u16, bool)>) {
    add_face(bytes, family, face, false);
}

/// raea: a face the app bundles for SVG text (raea-sys-fonts' faces), found by `family` until a page gives its own
#[cfg(feature = "svg")]
pub fn add_bundled_svg_font(bytes: Vec<u8>, family: Option<&str>, face: Option<(u16, bool)>) {
    add_face(bytes, family, face, true);
}

#[cfg(feature = "svg")]
fn add_face(bytes: Vec<u8>, family: Option<&str>, face: Option<(u16, bool)>, bundled: bool) {
    // once each: a face given again (document.fonts.load and the page's @font-face both give Google's) would be two
    // faces of one weight and style, which canvas text takes for a family split by unicode-range (raea-sys-canvas
    // subsets.rs)
    {
        use std::hash::{Hash, Hasher};
        static SEEN: LazyLock<Mutex<std::collections::HashSet<u64>>> = LazyLock::new(Default::default);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&bytes, family, face).hash(&mut h);
        if !SEEN.lock().unwrap().insert(h.finish()) {
            return;
        }
    }
    // the families the page has given faces of, in lower case
    static PAGE: LazyLock<Mutex<std::collections::HashSet<String>>> = LazyLock::new(Default::default);
    let mut page = PAGE.lock().unwrap();
    // a bundled face of a family the page already has isn't added: raea/text's loadFont fetches Google's Geist when
    // the composition's module runs, which can be before the app's bundled Geist is loaded
    if let (true, Some(family)) = (bundled, family) {
        if page.contains(&family.to_lowercase()) {
            return;
        }
    }
    let mut db = FONT_DB.lock().unwrap();
    let db = Arc::make_mut(&mut db);
    let mut bundled_ids = BUNDLED.lock().unwrap();
    // the page's own faces of a family: the bundled ones of that name go
    if let (false, Some(family)) = (bundled, family) {
        page.insert(family.to_lowercase());
        for id in bundled_ids.remove(&family.to_lowercase()).unwrap_or_default() {
            db.remove_face(id);
        }
    }
    let mut ids = db.load_font_source(fontdb::Source::Binary(Arc::new(bytes))).to_vec();
    if family.is_some() || face.is_some() {
        for id in std::mem::take(&mut ids) {
            if let Some(mut info) = db.face(id).cloned() {
                db.remove_face(id);
                if let Some(family) = family {
                    info.families.insert(0, (family.to_string(), fontdb::Language::English_UnitedStates));
                }
                if let Some((weight, italic)) = face {
                    info.weight = fontdb::Weight(weight);
                    info.style = if italic { fontdb::Style::Italic } else { fontdb::Style::Normal };
                }
                ids.push(db.push_face_info(info));
            }
        }
    }
    if let (true, Some(family)) = (bundled, family) {
        bundled_ids.entry(family.to_lowercase()).or_default().extend(ids);
    }
}

/// Which kind of CSS image layer list (`background-image` or `mask-image`) to
/// flush from style to dedicated storage on the node.
#[derive(Clone, Copy, Debug)]
pub enum ImageLayerKind {
    Background,
    Mask,
    /// raea: border-image-source, one layer (bevy_ui's NodeImageMode::Sliced)
    Border,
}

impl ImageLayerKind {
    pub fn image_type(self, idx: usize) -> ImageType {
        match self {
            Self::Background => ImageType::Background(idx),
            Self::Mask => ImageType::Mask(idx),
            Self::Border => ImageType::Border(idx),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ImageType {
    Image,
    Background(usize),
    Mask(usize),
    /// raea: the border-image-source
    Border(usize),
}

/// A point
#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub struct Point<T> {
    /// The x coordinate
    pub x: T,
    /// The y coordinate
    pub y: T,
}

impl Point<f64> {
    pub const ZERO: Self = Point { x: 0.0, y: 0.0 };
}

// Debug print an RcDom
pub fn walk_tree(indent: usize, node: &Node) {
    // Skip all-whitespace text nodes entirely
    if let NodeData::Text(data) = &node.data {
        if data.content.chars().all(|c| c.is_ascii_whitespace()) {
            return;
        }
    }

    print!("{}", " ".repeat(indent));
    let id = node.id;
    match &node.data {
        NodeData::Document(_) => println!("#Document {id}"),

        NodeData::Text(data) => {
            if data.content.chars().all(|c| c.is_ascii_whitespace()) {
                println!("{id} #text: <whitespace>");
            } else {
                let content = data.content.trim();
                if content.len() > 10 {
                    println!(
                        "#text {id}: {}...",
                        content
                            .split_at(content.char_indices().take(10).last().unwrap().0)
                            .0
                            .escape_default()
                    )
                } else {
                    println!("#text {id}: {}", data.content.trim().escape_default())
                }
            }
        }

        NodeData::Comment { .. } => println!("<!-- COMMENT {id} -->"),

        NodeData::AnonymousBlock(_) => println!("{id} AnonymousBlock"),

        NodeData::Element(data) => {
            print!("<{} {id}", data.name.local);
            for attr in data.attrs.iter() {
                print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            if !node.children.is_empty() {
                println!(">");
            } else {
                println!("/>");
            }
        } // NodeData::Doctype {
          //     ref name,
          //     ref public_id,
          //     ref system_id,
          // } => println!("<!DOCTYPE {} \"{}\" \"{}\">", name, public_id, system_id),
          // NodeData::ProcessingInstruction { .. } => unreachable!(),
    }

    if !node.children.is_empty() {
        for child_id in node.children.iter() {
            walk_tree(indent + 2, node.with(*child_id));
        }

        if let NodeData::Element(data) = &node.data {
            println!("{}</{}>", " ".repeat(indent), data.name.local);
        }
    }
}

/// raea: the faces SVG text is drawn from (the system's, and the page's own from `add_svg_font`), for a canvas's text too
#[cfg(feature = "svg")]
pub fn svg_fonts() -> Arc<fontdb::Database> {
    Arc::clone(&FONT_DB.lock().unwrap())
}

/// raea: an SVG image's own `@font-face` faces given as data: URLs (what Excalidraw's exportToSvg embeds), each as
/// (font-family, bytes). usvg skips `@font-face` ("not supported"), so its text fell back to the default face.
#[cfg(feature = "svg")]
fn svg_font_faces(source: &[u8]) -> Vec<(String, Vec<u8>)> {
    let Ok(text) = std::str::from_utf8(source) else { return Vec::new() };
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("@font-face") {
        rest = &rest[at + 10..];
        let Some(open) = rest.find('{') else { break };
        let Some(close) = rest[open..].find('}') else { break };
        let body = &rest[open + 1..open + close];
        rest = &rest[open + close..];
        let family = body.split(';').find_map(|d| {
            let (k, v) = d.split_once(':')?;
            (k.trim().eq_ignore_ascii_case("font-family")).then(|| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        });
        let Some(family) = family.filter(|f| !f.is_empty()) else { continue };
        // each url(data:...;base64,...) in src: the first that decodes is the face
        let mut src = body;
        while let Some(u) = src.find("url(") {
            src = &src[u + 4..];
            let end = src.find(')').unwrap_or(src.len());
            let url = src[..end].trim().trim_matches(|c| c == '"' || c == '\'');
            src = &src[end..];
            let Some((meta, data)) = url.strip_prefix("data:").and_then(|u| u.split_once(',')) else { continue };
            if !meta.split(';').any(|p| p.trim().eq_ignore_ascii_case("base64")) {
                continue;
            }
            if let Some(bytes) = base64_decode(data) {
                out.push((family.clone(), decode_font_bytes(&bytes).into_owned()));
                break;
            }
        }
    }
    out
}

/// Base64 as a data: URL has it (either alphabet; whitespace and padding skipped). None on any other byte.
#[cfg(feature = "svg")]
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Parse an SVG image.
#[cfg(feature = "svg")]
pub(crate) fn parse_svg_image(source: &[u8]) -> Result<crate::node::SvgImageData, usvg::Error> {
    // raea: its own @font-face faces join the SVG faces first (add_svg_font takes each once), so its text is drawn in them
    for (family, bytes) in svg_font_faces(source) {
        add_svg_font(bytes, Some(&family), None);
    }
    let options = usvg::Options {
        fontdb: Arc::clone(&FONT_DB.lock().unwrap()),
        ..Default::default()
    };
    crate::node::SvgImageData::from_data(source, &options)
}

pub trait ToColorColor {
    /// Converts a color into the `AlphaColor<Srgb>` type from the `color` crate
    fn as_color_color(&self) -> Color;
}
impl ToColorColor for AbsoluteColor {
    fn as_color_color(&self) -> Color {
        Color::new(
            *self
                .to_color_space(style::color::ColorSpace::Srgb)
                .raw_components(),
        )
    }
}

#[cfg(all(test, feature = "svg"))]
mod svg_tests {
    use super::parse_svg_image;

    /// raea: a face is found by its @font-face weight, not its file's, and a page's faces of a family shadow the app's
    /// bundled ones (Google's variable Inter beside raea's bundled static Inter drew 500 as 400, layout-reads LR10)
    #[cfg(target_os = "macos")]
    #[test]
    fn an_svg_face_is_found_at_its_font_face_weight() {
        use super::{add_bundled_svg_font, add_svg_font, fontdb, svg_fonts};
        let family = "raea lr10 weight";
        let stat = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap();
        let var = std::fs::read("/System/Library/Fonts/SFNS.ttf").unwrap();
        add_bundled_svg_font(stat, Some(family), Some((400, false)));
        let named = |db: &fontdb::Database| db.faces().filter(|f| f.families.iter().any(|(n, _)| n == family)).count();
        assert_eq!(named(&svg_fonts()), 1, "the bundled face, until the page gives its own");
        for w in [400, 500, 600] {
            add_svg_font(var.clone(), Some(family), Some((w, false)));
        }
        let db = svg_fonts();
        assert_eq!(named(&db), 3, "the page's three; the bundled one has gone");
        let at = |w: u16| {
            let q = fontdb::Query { families: &[fontdb::Family::Name(family)], weight: fontdb::Weight(w), ..Default::default() };
            let id = db.query(&q).unwrap();
            let face = db.face(id).unwrap();
            (face.weight.0, face.post_script_name.clone())
        };
        let (w, name) = at(500);
        assert_eq!(w, 500, "{name}");
        assert!(!name.contains("Arial"), "500 found the static 400: {name}");
        assert_eq!(at(600).0, 600);
        // given again, it isn't a second face of that weight
        let count = |db: &fontdb::Database| db.faces().filter(|f| f.families.iter().any(|(n, _)| n == family) && f.weight.0 == 500).count();
        assert_eq!(count(&db), 1);
        add_svg_font(var, Some(family), Some((500, false)));
        assert_eq!(count(&svg_fonts()), 1);
    }

    /// raea: a bundled face of a family the page gave first isn't added beside the page's (loadFont fetches at import)
    #[cfg(target_os = "macos")]
    #[test]
    fn a_bundled_face_after_the_page_s_own_is_left_out() {
        use super::{add_bundled_svg_font, add_svg_font, svg_fonts};
        let family = "raea lr10 page first";
        let var = std::fs::read("/System/Library/Fonts/SFNS.ttf").unwrap();
        let stat = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap();
        add_svg_font(var, Some(family), Some((400, false)));
        add_bundled_svg_font(stat, Some(family), Some((400, false)));
        let names: Vec<_> = svg_fonts().faces().filter(|f| f.families.iter().any(|(n, _)| n == family)).map(|f| f.post_script_name.clone()).collect();
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(!names[0].contains("Arial"), "{names:?}");
    }

    #[test]
    fn svg_without_xmlns_parses() {
        let src = br#"<svg viewBox="0 0 24 24"><path d="M0 0h24v24H0z"/></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert!(!svg.tree.root().children().is_empty());
    }

    #[test]
    fn missing_height_is_computed_from_width_and_viewbox_ratio() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="200"><rect width="100%" height="100%" fill="green"/></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert_eq!(svg.intrinsic_width(), Some(200.0));
        assert_eq!(svg.intrinsic_height(), None);
        assert_eq!(svg.viewbox_aspect_ratio(), Some(1.0));
        assert_eq!(svg.tree.size().width(), 200.0);
        assert_eq!(svg.intrinsic_size(), (200.0, 200.0));
    }

    #[test]
    fn viewbox_only_has_no_intrinsic_dimensions() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 485 58"></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert_eq!(svg.intrinsic_width(), None);
        assert_eq!(svg.intrinsic_height(), None);
        // The aspect ratio is still available from the viewBox.
        assert!((svg.aspect_ratio() - (485.0 / 58.0)).abs() < 1e-3);
    }

    #[test]
    fn absolute_dimensions_are_intrinsic() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16" viewBox="0 0 48 32"></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert_eq!(svg.intrinsic_width(), Some(24.0));
        assert_eq!(svg.intrinsic_height(), Some(16.0));
    }

    #[test]
    fn percentage_dimensions_are_not_intrinsic() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="50%" viewBox="0 0 200 100"></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert_eq!(svg.intrinsic_width(), None);
        assert_eq!(svg.intrinsic_height(), None);
    }

    #[test]
    fn unit_lengths_are_intrinsic() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24px" height="1.5em" viewBox="0 0 48 32"></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert!(svg.intrinsic_width().is_some());
        assert!(svg.intrinsic_height().is_some());
    }

    #[test]
    fn non_numeric_dimensions_are_not_intrinsic() {
        let src = br#"<svg xmlns="http://www.w3.org/2000/svg" width="auto" height="foo" viewBox="0 0 200 100"></svg>"#;
        let svg = parse_svg_image(src).unwrap();
        assert_eq!(svg.intrinsic_width(), None);
        assert_eq!(svg.intrinsic_height(), None);
    }
}

/// Creates an markup5ever::QualName.
/// Given a local name and an optional namespace
#[macro_export]
macro_rules! qual_name {
    ($local:tt $(, $ns:ident)?) => {
        $crate::QualName {
            prefix: None,
            ns: $crate::ns!($($ns)?),
            local: $crate::local_name!($local),
        }
    };
}

/// raea: the lines a `-webkit-line-clamp` keeps: a clamped vertical `-webkit-box` (the vendored stylo's style adjuster
/// makes it a flow root) ends with its Nth line, which blitz-paint draws with an ellipsis
pub fn clamped_lines(style: &style::properties::ComputedValues) -> Option<usize> {
    use style::values::generics::Optional;
    let clamp = style.get_box().clone_line_clamp();
    match &clamp.max_lines.lines {
        Optional::Some(n) => Some(n.0.max(1) as usize),
        Optional::None => None,
    }
}
