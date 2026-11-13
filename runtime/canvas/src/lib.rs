//! A composition's 2D context answered by the host (docs/layout-reads.feature LR6): the calls an app page's canvas makes
//! of its host (`canvas.measure`, `canvas.advances`, `canvas.read`), as native calls. The context (@raea/ui's) asks
//! through its wire's synchronous call, which a composition's entry routes here (sdk/raea/src/canvas2d.ts), so
//! measureText is shaped from the fonts the page draws with and getImageData reads back what the context drew.

/// The calls, registered with the composition's other natives (raea-runtime-react's thread.rs).
pub fn register() {
    // the faces a canvas's text is shaped in: the page's (blitz-dom adds every face the composition loads)
    raea_sys_canvas::fonts(blitz_dom::util::svg_fonts);
    // the text natives, each laid out in the page's fonts as they are when it's called
    let text: [(&str, &str, &str, &str, raea_sys_canvas::TextNative); 3] = [
        (
            "canvas.measure",
            "{ font: { family: string; size: number; weight: string; style: string }; text: string; align?: string; baseline?: string }",
            "{ width: number; actualBoundingBoxLeft: number; actualBoundingBoxRight: number; actualBoundingBoxAscent: number; actualBoundingBoxDescent: number; fontBoundingBoxAscent: number; fontBoundingBoxDescent: number }",
            "measureText: the text shaped as the canvas draws it, in the page's fonts; its advance, ink and font box from the alignment point (docs/layout-reads.feature LR6)",
            raea_sys_canvas::metrics,
        ),
        (
            "canvas.svgText",
            "{ text: string; style: Record<string, string> }",
            "{ length: number; starts: number[] }",
            "an svg <text>'s getComputedTextLength and getStartPositionOfChar: its words laid out by usvg, as the page draws an svg, in its computed font (font-*, letter-spacing, word-spacing); x from the text's own (docs/layout-reads.feature LR7)",
            raea_sys_canvas::svg_text,
        ),
        (
            "canvas.advances",
            "{ font: { family: string; weight: string; style: string }; chars: string }",
            "{ adv: (number | null)[]; own: boolean[] }",
            "each character's advance (a fraction of the font size) in the face a canvas draws it with, and whether the font's own family has it",
            raea_sys_canvas::advances,
        ),
    ];
    for (name, args, answer, doc, f) in text {
        raea_native::register(name, args, answer, doc, move |a| raea_sys_canvas::with_page_fonts(f, a));
    }
    raea_native::register(
        "fonts.load",
        "{ family: string }",
        "{ faces: number }",
        "document.fonts.load: a family the composition asked for (loadFont), its faces given to a canvas's text now, where the page fetches it only once an element names it (docs/layout-reads.feature LR7); how many faces it has",
        |a| {
            let family = a["family"].as_str().ok_or("fonts.load: no family")?;
            Ok(serde_json::json!({ "faces": raea_sys_fonts::google::load_now(family) }))
        },
    );
    raea_native::register(
        "fonts.face",
        "{ family: string; url: string }",
        "{ faces: number }",
        "FontFace.load for a face given by URL: fetched now, through raea's cache, into the faces an svg's and a canvas's text are set in, as `family` (docs/layout-reads.feature LR7); 0 faces when it doesn't load",
        |a| {
            let (Some(family), Some(url)) = (a["family"].as_str(), a["url"].as_str()) else { return Err("fonts.face: a family and a url".into()) };
            Ok(serde_json::json!({ "faces": raea_sys_fonts::google::face_now(family, url) }))
        },
    );
    raea_native::register(
        "canvas.read",
        "{ width: number; height: number; __canvas2d: unknown; x: number; y: number; w: number; h: number }",
        "{ data: string }",
        "getImageData: a canvas's display list drawn on a bitmap of its own, and a rectangle of it read off as base64 RGBA",
        |a| raea_sys_canvas::read(a),
    );
}
