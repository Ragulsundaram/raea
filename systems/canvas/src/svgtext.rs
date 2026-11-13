//! An SVG <text>'s measures (docs/layout-reads.feature LR7): getComputedTextLength and getStartPositionOfChar, as usvg
//! lays its words out in the element's computed font, the engine and fonts the page draws an svg with. A composition
//! reads them off hidden text (an opacity-0 group) to set its words, which the page's svg never lays out, so the words
//! are laid out here on their own.

use serde_json::{json, Value};

/// The computed properties that change where an svg text's glyphs fall.
const FONT: [&str; 9] = ["font-family", "font-size", "font-weight", "font-style", "font-stretch", "letter-spacing", "word-spacing", "font-variation-settings", "font-feature-settings"];

/// `canvas.svgText`: {text, style: {property: computed value}} -> {length, starts}: the text's advance
/// (getComputedTextLength) and the x where each of its characters starts (getStartPositionOfChar), in user units from
/// the text's own x. Spaces are kept, as `white-space: pre` keeps them.
pub fn svg_text(opt: &usvg::Options, args: &Value) -> Value {
    let text = args["text"].as_str().unwrap_or("");
    let style: Vec<String> = FONT
        .iter()
        .filter_map(|p| args["style"][p].as_str().filter(|v| !v.is_empty()).map(|v| format!("{p}:{}", v.replace('"', "'").replace('&', "&amp;"))))
        .collect();
    let esc = text.replace('&', "&amp;").replace('<', "&lt;");
    let doc = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><text x=\"0\" y=\"0\" xml:space=\"preserve\" style=\"{}\">{esc}</text></svg>", style.join(";"));
    let t = usvg::Tree::from_str(&doc, opt).ok();
    let t = t.as_ref().and_then(|t| t.root().children().iter().find_map(|n| if let usvg::Node::Text(t) = n { Some(t.clone()) } else { None }));
    let Some(t) = t else { return json!({ "length": 0, "starts": vec![0; text.chars().count()] }) };
    let mut starts = Vec::new();
    for span in t.layouted() {
        for g in &span.positioned_glyphs {
            // a ligature's characters all start where it does
            let x = g.transform().tx;
            starts.extend(g.text.chars().map(|_| x));
        }
    }
    // usvg's box ends at the last glyph's advance; Chrome's length tracks every character, the last one too
    // (a computed letter-spacing is px, or "normal", which is none)
    let tracking: f32 = match args["style"]["letter-spacing"].as_str() {
        Some(v) if v.ends_with("px") => v.trim_end_matches("px").trim().parse().unwrap_or(0.0),
        _ => 0.0,
    };
    let length = if text.is_empty() { 0.0 } else { t.bounding_box().width() + tracking };
    json!({ "length": length, "starts": starts })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_texts_length_is_its_advance_and_tracking_widens_it() {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap());
        let opt = usvg::Options { fontdb: std::sync::Arc::new(db), ..Default::default() };
        let st = |ls: &str| json!({ "font-family": "Arial", "font-size": "40px", "letter-spacing": ls });
        let plain = super::svg_text(&opt, &json!({ "text": "Hi there", "style": st("normal") }));
        let wide = super::svg_text(&opt, &json!({ "text": "Hi there", "style": st("10px") }));
        let (a, b) = (plain["length"].as_f64().unwrap(), wide["length"].as_f64().unwrap());
        assert!(a > 100.0, "{plain}");
        assert!((b - a - 80.0).abs() < 1.0, "8 characters tracked 10 px each: {a} -> {b}");
        let s = plain["starts"].as_array().unwrap();
        assert_eq!(s.len(), 8, "one start per character, the space's too: {plain}");
        assert_eq!(s[0].as_f64(), Some(0.0));
        assert!(s.windows(2).all(|w| w[1].as_f64() > w[0].as_f64()), "{plain}");
    }
}
