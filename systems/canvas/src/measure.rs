//! A canvas's text as usvg shapes it, for drawing (draw.rs) and measuring. measureText is `canvas.measure`: the text
//! shaped as the canvas would draw it, its advance and its ink (docs/layout-reads.feature LR6). An app page without it
//! asks `canvas.advances`, each character's advance in the face the canvas would draw it with: the characters it hasn't
//! measured yet in a font, keeping the ones its own family has, so a face the page loads later takes over.

use serde_json::{json, Value};
use usvg::fontdb::{Family, Query, Style, Weight, ID};

/// A 2D context's text call as the SVG usvg shapes: `text` at `at` in `font` ({family, size, weight, style}), on the
/// context's (textAlign, textBaseline), painted by `paint` (attributes), on a `size` document.
pub(crate) fn text_doc(opt: &usvg::Options, font: &Value, text: &str, at: (f32, f32), (align, baseline): (&Value, &Value), paint: &str, (w, h): (u32, u32)) -> String {
    let anchor = match align.as_str() {
        Some("center") => "middle",
        Some("right") => "end",
        _ => "start",
    };
    let base = match baseline.as_str() {
        Some("top") => "text-before-edge",
        Some("hanging") => "hanging",
        Some("middle") => "middle",
        Some("bottom") => "text-after-edge",
        Some("ideographic") => "ideographic",
        _ => "alphabetic",
    };
    let escape = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;");
    let fam = font["family"].as_str().unwrap_or("sans-serif").replace('"', "'");
    // a family split across files by unicode-range: each run names the file that has its glyphs (subsets.rs)
    let split = crate::split_families(&opt.fontdb).and_then(|s| crate::subsets::runs(&opt.fontdb, &s, &fam, text));
    let esc = match split {
        Some(runs) => runs.iter().map(|(f, s)| match f {
            Some(f) => format!("<tspan font-family=\"'{}', {fam}\">{}</tspan>", f.replace('\'', ""), escape(s)),
            None => format!("<tspan>{}</tspan>", escape(s)),
        }).collect(),
        None => escape(text),
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\"><text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"{fam}\" font-weight=\"{}\" font-style=\"{}\" text-anchor=\"{anchor}\" dominant-baseline=\"{base}\" {paint}>{esc}</text></svg>",
        at.0,
        at.1,
        crate::paint::f(&font["size"]),
        font["weight"].as_str().unwrap_or("400"),
        font["style"].as_str().unwrap_or("normal"),
    )
}

/// `canvas.measure`: {font, text, align, baseline} -> TextMetrics (width, actualBoundingBox{Left,Right,Ascent,Descent},
/// fontBoundingBox{Ascent,Descent}): the text shaped as the canvas draws it, from the alignment point. Its advance and
/// the font's ascent and descent are usvg's layout box; the ink is its glyphs' outlines. Spaces are kept, as fillText
/// keeps them.
pub fn metrics(args: &Value) -> Result<Value, String> {
    Ok(metrics_in(&crate::page_options(), args))
}

fn metrics_in(opt: &usvg::Options, args: &Value) -> Value {
    let text = args["text"].as_str().unwrap_or("");
    let shape = |text: &str| {
        let doc = text_doc(opt, &args["font"], text, (0.0, 0.0), (&args["align"], &args["baseline"]), "xml:space=\"preserve\" fill=\"#fff\"", (1, 1));
        let tree = usvg::Tree::from_str(&doc, opt).ok()?;
        tree.root().children().iter().find_map(|n| if let usvg::Node::Text(t) = n { Some((t.bounding_box(), t.flattened().bounding_box())) } else { None })
    };
    let none = usvg::Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap();
    let (lay, ink, width) = match shape(text) {
        Some((lay, ink)) => (lay, ink, lay.width()),
        // nothing usvg draws (empty, or spaces alone, which it drops): no ink, the characters' advances, and the font's
        // box from a glyph it does draw
        None => {
            let adv = advances_in(opt, &json!({ "font": args["font"], "chars": text }));
            let em: f64 = adv["adv"].as_array().into_iter().flatten().filter_map(Value::as_f64).sum();
            let lay = if text.is_empty() { none } else { shape("x").map_or(none, |(lay, _)| lay) };
            (lay, none, em as f32 * crate::paint::f(&args["font"]["size"]))
        }
    };
    json!({
        "width": width,
        "actualBoundingBoxLeft": -ink.left(),
        "actualBoundingBoxRight": ink.right(),
        "actualBoundingBoxAscent": -ink.top(),
        "actualBoundingBoxDescent": ink.bottom(),
        "fontBoundingBoxAscent": -lay.top(),
        "fontBoundingBoxDescent": lay.bottom(),
    })
}

/// `canvas.advances`: {font: {family, weight, style}, chars} -> {adv: [em or null], own: [bool]}, one per character:
/// its advance as a fraction of the font size, and whether the font's first family has it.
pub fn advances(args: &Value) -> Result<Value, String> {
    Ok(advances_in(&crate::page_options(), args))
}

fn advances_in(opt: &usvg::Options, args: &Value) -> Value {
    let db = &opt.fontdb;
    let font = &args["font"];
    let weight = font["weight"].as_str().and_then(|w| w.parse().ok()).unwrap_or(400u16);
    let style = if font["style"].as_str() == Some("italic") { Style::Italic } else { Style::Normal };
    let names: Vec<String> = font["family"].as_str().unwrap_or("sans-serif").split(',').map(|s| s.trim().trim_matches(['\'', '"']).to_string()).filter(|s| !s.is_empty()).collect();
    let face_of = |name: &str| -> Option<ID> {
        let family = match name.to_lowercase().as_str() {
            "serif" => Family::Serif,
            "sans-serif" | "system-ui" => Family::SansSerif,
            "monospace" => Family::Monospace,
            "cursive" => Family::Cursive,
            "fantasy" => Family::Fantasy,
            _ => Family::Name(name),
        };
        db.query(&Query { families: &[family], weight: Weight(weight), style, ..Default::default() })
    };
    // the faces in the order a character is looked for: a split first family's subsets (subsets.rs), the families as
    // listed, then the default
    let mut faces: Vec<(ID, bool)> = Vec::new();
    if let (Some(split), Some(first)) = (crate::split_families(db), names.first()) {
        faces.extend(split.get(&first.to_lowercase()).into_iter().flatten().map(|(_, id)| (*id, true)));
    }
    faces.extend(names.iter().enumerate().filter_map(|(k, n)| face_of(n).map(|id| (id, k == 0))));
    faces.extend(face_of("sans-serif").map(|id| (id, false)));
    let chars: Vec<char> = args["chars"].as_str().unwrap_or("").chars().collect();
    let mut out: Vec<Option<(f64, bool)>> = vec![None; chars.len()];
    for (id, own) in faces {
        if out.iter().all(Option::is_some) {
            break;
        }
        db.with_face_data(id, |data, index| {
            let Ok(f) = ttf_parser::Face::parse(data, index) else { return };
            let em = f64::from(f.units_per_em());
            for (c, slot) in chars.iter().zip(out.iter_mut()).filter(|(_, s)| s.is_none()) {
                if let Some(a) = f.glyph_index(*c).and_then(|g| f.glyph_hor_advance(g)) {
                    *slot = Some((f64::from(a) / em, own));
                }
            }
        });
    }
    json!({
        "adv": out.iter().map(|s| s.map(|(a, _)| a)).collect::<Vec<_>>(),
        "own": out.iter().map(|s| s.is_some_and(|(_, o)| o)).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_are_the_faces_and_say_whose() {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap());
        db.set_sans_serif_family("Arial");
        let opt = usvg::Options { fontdb: std::sync::Arc::new(db), ..Default::default() };
        let r = advances_in(&opt, &json!({"font": {"family": "Arial", "weight": "400", "style": "normal"}, "chars": "im"}));
        let (i, m) = (r["adv"][0].as_f64().unwrap(), r["adv"][1].as_f64().unwrap());
        assert!(i < 0.3 && m > 0.8, "{r}");
        assert_eq!(r["own"], json!([true, true]));
        // a family the page doesn't have: the default stands in, and says so
        let r = advances_in(&opt, &json!({"font": {"family": "Nope, sans-serif", "weight": "400", "style": "normal"}, "chars": "i"}));
        assert_eq!(r["own"], json!([false]));
        assert!((r["adv"][0].as_f64().unwrap() - i).abs() < 1e-9);
        // measureText: the advance is the characters' (Arial has no kerning between these), the ink the glyphs'
        let mt = |text: &str, align: &str| metrics_in(&opt, &json!({"font": {"family": "Arial", "size": 100, "weight": "400", "style": "normal"}, "text": text, "align": align, "baseline": "alphabetic"}));
        let (x, h) = (mt("x", "left"), mt("H", "left"));
        let g = |v: &Value, k: &str| v[k].as_f64().unwrap();
        assert!((g(&mt("im", "left"), "width") - (i + m) * 100.0).abs() < 0.01, "{}", mt("im", "left"));
        assert!(g(&x, "actualBoundingBoxAscent") > 45.0 && g(&x, "actualBoundingBoxAscent") < g(&h, "actualBoundingBoxAscent"), "{x} {h}");
        assert!(g(&mt("g", "left"), "actualBoundingBoxDescent") > 15.0 && g(&h, "actualBoundingBoxDescent").abs() < 0.5);
        assert!(g(&h, "fontBoundingBoxAscent") > g(&h, "actualBoundingBoxAscent"));
        // right-aligned, the ink is left of the alignment point; spaces keep their advance and have no ink
        assert!(g(&mt("H", "right"), "actualBoundingBoxLeft") > 60.0);
        assert!(g(&mt("  ", "left"), "width") > 50.0 && g(&mt("  ", "left"), "actualBoundingBoxRight") == 0.0);
        assert!((g(&mt(" H ", "left"), "width") - g(&h, "width") - g(&mt("  ", "left"), "width")).abs() < 0.01, "{}", mt(" H ", "left"));
        assert_eq!(g(&mt("", "left"), "width"), 0.0);
    }
}
