//! A composition's 2D context answered by the host (docs/layout-reads.feature LR6): the calls an app page's canvas makes
//! of its host (`canvas.measure`, `canvas.advances`, `canvas.read`), as native calls. The context (@raea/ui's) asks
//! through its wire's synchronous call, which a composition's entry routes here (sdk/raea/src/canvas2d.ts), so
//! measureText is shaped from the fonts the page draws with and getImageData reads back what the context drew.

/// The calls, registered with the composition's other natives (raea-runtime-react's thread.rs).
pub fn register() {
    // the faces a canvas's text is shaped in: the page's (blitz-dom adds every face the composition loads)
    raea_sys_canvas::fonts(blitz_dom::util::svg_fonts);
    raea_native::register(
        "canvas.measure",
        "{ font: { family: string; size: number; weight: string; style: string }; text: string; align?: string; baseline?: string }",
        "{ width: number; actualBoundingBoxLeft: number; actualBoundingBoxRight: number; actualBoundingBoxAscent: number; actualBoundingBoxDescent: number; fontBoundingBoxAscent: number; fontBoundingBoxDescent: number }",
        "measureText: the text shaped as the canvas draws it, in the page's fonts; its advance, ink and font box from the alignment point (docs/layout-reads.feature LR6)",
        |a| raea_sys_canvas::metrics(a),
    );
    raea_native::register(
        "canvas.advances",
        "{ font: { family: string; weight: string; style: string }; chars: string }",
        "{ adv: (number | null)[]; own: boolean[] }",
        "each character's advance (a fraction of the font size) in the face a canvas draws it with, and whether the font's own family has it",
        |a| raea_sys_canvas::advances(a),
    );
    raea_native::register(
        "canvas.read",
        "{ width: number; height: number; __canvas2d: unknown; x: number; y: number; w: number; h: number }",
        "{ data: string }",
        "getImageData: a canvas's display list drawn on a bitmap of its own, and a rectangle of it read off as base64 RGBA",
        |a| raea_sys_canvas::read(a),
    );
}
