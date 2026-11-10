//! Serialization of nodes to HTML, and of inline `<svg>` subtrees to SVG source for usvg.

use html_escape::{encode_quoted_attribute_to_string, encode_text_to_string};
use markup5ever::{QualName, local_name, ns};

use super::{Node, NodeData};

#[derive(Clone, Copy)]
enum OutputStyle {
    Normal,
    Pretty,
}

#[derive(Clone, Copy)]
struct SerializeOpts {
    style: OutputStyle,
    /// Replace `currentColor` in attribute values with the element's computed `color`
    resolve_current_color: bool,
    /// Declare the `xlink` prefix on the root element so that the output is well-formed XML
    declare_xlink_ns: bool,
}

impl SerializeOpts {
    const fn html(style: OutputStyle) -> Self {
        Self {
            style,
            resolve_current_color: false,
            declare_xlink_ns: false,
        }
    }
}

const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// Writes an attribute's serialized name, per the HTML fragment serialization algorithm
fn write_attr_name(name: &QualName, writer: &mut String) {
    match name.ns {
        ns!() | ns!(html) => {}
        ns!(xml) => writer.push_str("xml:"),
        ns!(xmlns) if name.local.as_str() == "xmlns" => {}
        ns!(xmlns) => writer.push_str("xmlns:"),
        ns!(xlink) => writer.push_str("xlink:"),
        _ => {
            if let Some(prefix) = &name.prefix {
                writer.push_str(prefix);
                writer.push(':');
            }
        }
    }
    writer.push_str(&name.local);
}

fn is_xlink_ns_decl(name: &QualName) -> bool {
    (name.ns == ns!(xmlns) && name.local.as_str() == "xlink")
        || name.local.as_str() == "xmlns:xlink"
}

impl Node {
    /// Renders the HTML of this node and all its children as a `String` without extra whitespace.
    ///
    /// Example output:
    ///
    /// ```text
    /// <html><head /><body><main id="main"><div class="arbitrary-class" /></main></body></html>
    /// ```
    pub fn outer_html(&self) -> String {
        let mut output = String::new();
        self.write_outer_html(&mut output);
        output
    }

    /// Renders the HTML of this node and all its children as a `String` with whitespace for human
    /// readability.
    ///
    /// Example output:
    ///
    /// ```text
    /// <html>
    ///   <head />
    ///   <body>
    ///     <main id="main">
    ///       <div class="arbitrary-class" />
    ///     </main>
    ///   </body>
    /// </html>
    /// ```
    pub fn outer_html_pretty(&self) -> String {
        let mut output = String::new();
        self.write_outer_html_pretty(&mut output);
        output
    }

    pub fn write_outer_html(&self, writer: &mut String) {
        self.write_outer_html_in_style(writer, SerializeOpts::html(OutputStyle::Normal), 0);
    }

    pub fn write_outer_html_pretty(&self, writer: &mut String) {
        self.write_outer_html_in_style(writer, SerializeOpts::html(OutputStyle::Pretty), 0);
    }

    /// Serializes this `<svg>` subtree as a standalone SVG document for usvg, with
    /// `currentColor` resolved against each element's computed `color`.
    #[cfg(feature = "svg")]
    pub(crate) fn svg_source(&self) -> String {
        let mut output = String::new();
        let opts = SerializeOpts {
            style: OutputStyle::Normal,
            resolve_current_color: true,
            declare_xlink_ns: true,
        };
        self.write_outer_html_in_style(&mut output, opts, 0);
        // raea: the root takes the font the page gives it (an inline svg's text is in its inherited font-family and
        // size, as in Chrome), unless it sets its own
        if let (Some(st), Some(el)) = (self.primary_styles(), self.element_data()) {
            let has = |n: &str| el.attrs.iter().any(|a| &*a.name.local == n);
            let mut extra = String::new();
            if !has("font-family") {
                use style::values::computed::font::{GenericFontFamily, SingleFontFamily};
                let list: Vec<String> = st.get_font().font_family.families.iter().map(|f| match f {
                    SingleFontFamily::FamilyName(n) => format!("'{}'", n.name.to_string().replace('\'', "")),
                    SingleFontFamily::Generic(g) => match g {
                        GenericFontFamily::Serif => "serif".into(),
                        GenericFontFamily::SansSerif => "sans-serif".into(),
                        GenericFontFamily::Monospace => "monospace".into(),
                        GenericFontFamily::Cursive => "cursive".into(),
                        GenericFontFamily::Fantasy => "fantasy".into(),
                        _ => "sans-serif".into(),
                    },
                }).collect();
                if !list.is_empty() {
                    extra.push_str(&format!(" font-family=\"{}\"", list.join(", ")));
                }
            }
            if !has("font-size") {
                extra.push_str(&format!(" font-size=\"{}\"", st.get_font().font_size.computed_size().px()));
            }
            if let Some(at) = output.find("<svg").map(|i| i + 4) {
                output.insert_str(at, &extra);
            }
        }
        output
    }

    /// raea: an svg attribute's value for usvg, when it needs resolving: its var()s substituted from this element's
    /// custom properties (Chrome reads `fill="var(--a6)"` as it would the property), and each colour usvg doesn't
    /// parse (oklch(), lab(), color-mix(), a resolved currentColor) as sRGB rgba(). A `style` attribute is resolved
    /// declaration by declaration. None when the value is fine as written.
    fn svg_attr_value(&self, name: &str, value: &str, current: Option<&str>) -> Option<String> {
        const UNREAD: [&str; 7] = ["var(", "oklch(", "oklab(", "lab(", "lch(", "color(", "color-mix("];
        if !UNREAD.iter().any(|f| value.contains(f)) {
            return None;
        }
        let one = |v: &str| {
            let v = self.substitute_vars(v, 0);
            let v = match current {
                Some(c) => v.replace("currentColor", c),
                None => v,
            };
            css_color_rgba(&v).unwrap_or(v)
        };
        if name == "style" {
            let decls: Vec<String> = value
                .split(';')
                .filter(|d| !d.trim().is_empty())
                .map(|d| match d.split_once(':') {
                    Some((p, v)) => format!("{}:{}", p, one(v.trim())),
                    None => d.to_string(),
                })
                .collect();
            return Some(decls.join(";"));
        }
        Some(one(value.trim()))
    }

    /// `value` with each `var(--name[, fallback])` replaced by this element's computed custom property, or the
    /// fallback when it has none.
    fn substitute_vars(&self, value: &str, depth: u8) -> String {
        let Some(at) = value.find("var(") else { return value.to_string() };
        if depth > 16 {
            return value.to_string();
        }
        let open = at + 4;
        let mut level = 1;
        let mut end = None;
        for (i, ch) in value[open..].char_indices() {
            match ch {
                '(' => level += 1,
                ')' => {
                    level -= 1;
                    if level == 0 {
                        end = Some(open + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else { return value.to_string() };
        let inner = &value[open..end];
        let (var, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };
        let found = var.strip_prefix("--").and_then(|n| {
            let styles = self.primary_styles()?;
            let v = styles.custom_properties().get_for_cssom(&style::Atom::from(n))?;
            Some(style_traits::ToCss::to_css_string(v))
        });
        let with = found.or(fallback.map(str::to_string)).unwrap_or_default();
        let out = format!("{}{}{}", &value[..at], with, &value[end + 1..]);
        self.substitute_vars(&out, depth + 1)
    }

    /// raea: an svg element's computed transform as SVG's `matrix()`, for usvg, which reads only the `transform`
    /// attribute: the cascade's (a stylesheet's or style attribute's transform over the attribute, which is a
    /// presentation hint), with translate/rotate/scale, and transform-origin resolved in its transform-box as Chrome
    /// does (css-transforms-1 #transform-box). Some("") when it computes to none or the identity.
    #[cfg(feature = "svg")]
    fn svg_css_transform(&self) -> Option<String> {
        use style::values::computed::{CSSPixelLength, TransformBox};
        let styles = self.primary_styles()?;
        let box_styles = styles.get_box();
        // an element without a CSS layout box uses fill-box for content-box and stroke-box for border-box; with
        // vector-effect: non-scaling-stroke, stroke-box is fill-box (css-transforms-1, csswg-drafts#9640; raea #50)
        let non_scaling = svg_computed(&styles, "vector-effect").is_some_and(|v| v == "non-scaling-stroke")
            || self.attr(markup5ever::LocalName::from("vector-effect")).is_some_and(|v| v.trim() == "non-scaling-stroke");
        let rect = match box_styles.transform_box {
            TransformBox::ViewBox => self.svg_view_box(),
            TransformBox::FillBox | TransformBox::ContentBox => self.svg_fill_box().unwrap_or_default(),
            TransformBox::StrokeBox | TransformBox::BorderBox if non_scaling => self.svg_fill_box().unwrap_or_default(),
            TransformBox::StrokeBox | TransformBox::BorderBox => {
                let r = self.svg_fill_box().unwrap_or_default();
                let half = self.svg_stroke_width() / 2.0;
                r.inflate(half, half)
            }
        };
        let reference_box = euclid::Rect::new(
            euclid::Point2D::new(CSSPixelLength::new(0.0), CSSPixelLength::new(0.0)),
            euclid::Size2D::new(CSSPixelLength::new(rect.width() as f32), CSSPixelLength::new(rect.height() as f32)),
        );
        let Some(m) = crate::resolve_2d_transform(box_styles, reference_box) else {
            return Some(String::new());
        };
        // the origin is in the box, the box in user space
        let at = kurbo::Affine::translate((rect.x0, rect.y0));
        let [a, b, c, d, e, f] = (at * m * at.inverse()).as_coeffs();
        Some(format!("matrix({a} {b} {c} {d} {e} {f})"))
    }

    /// The nearest svg viewport's reference box in this element's user space: the `viewBox`'s size (or the viewport's)
    /// at the origin of user space, where css-transforms-1 #transform-box puts it. A viewBox's x and y move the
    /// content, not the box: placed at them, the box moved a 0 0 origin there, and `scale(2)` under
    /// `viewBox="100 50 400 300"` drew its content 100 px left of and 50 px above where Chrome draws it.
    #[cfg(feature = "svg")]
    fn svg_view_box(&self) -> kurbo::Rect {
        let mut at = self.parent;
        while let Some(id) = at {
            let node = &self.tree()[id];
            let Some(el) = node.element_data() else { break };
            if el.name.ns == ns!(svg) && &*el.name.local == "svg" {
                if let Some(vb) = el.attr(local_name!("viewBox")).and_then(|v| v.parse::<svgtypes::ViewBox>().ok()) {
                    return kurbo::Rect::new(0.0, 0.0, vb.w, vb.h);
                }
                let outer = node.parent.and_then(|p| self.tree()[p].element_data()).is_none_or(|p| p.name.ns != ns!(svg));
                if outer {
                    let size = node.final_layout().content_box_size();
                    return kurbo::Rect::new(0.0, 0.0, size.width as f64, size.height as f64);
                }
                let (w, h) = (node.svg_number("width"), node.svg_number("height"));
                if let (Some(w), Some(h)) = (w, h) {
                    return kurbo::Rect::new(0.0, 0.0, w, h);
                }
            }
            at = node.parent;
        }
        kurbo::Rect::ZERO
    }

    /// The element's object bounding box in its user space (its geometry, before its own transform); a container's
    /// is its children's union. None when it has no geometry.
    #[cfg(feature = "svg")]
    fn svg_fill_box(&self) -> Option<kurbo::Rect> {
        use kurbo::{Rect, Shape};
        let el = self.element_data()?;
        let n = |name: &str| self.svg_number(name).unwrap_or(0.0);
        match &*el.name.local {
            "rect" | "image" | "foreignObject" | "use" | "svg" => Some(Rect::new(n("x"), n("y"), n("x") + n("width"), n("y") + n("height"))),
            "circle" => Some(Rect::new(n("cx") - n("r"), n("cy") - n("r"), n("cx") + n("r"), n("cy") + n("r"))),
            "ellipse" => Some(Rect::new(n("cx") - n("rx"), n("cy") - n("ry"), n("cx") + n("rx"), n("cy") + n("ry"))),
            "line" => Some(Rect::from_points((n("x1"), n("y1")), (n("x2"), n("y2")))),
            "polyline" | "polygon" => {
                let points = el.attr(local_name!("points"))?;
                svgtypes::PointsParser::from(points).map(|(x, y)| Rect::from_points((x, y), (x, y))).reduce(|a, b| a.union(b))
            }
            "path" => Some(kurbo::BezPath::from_svg(el.attr(local_name!("d"))?).ok()?.bounding_box()),
            _ => self.children.iter().filter_map(|&c| self.tree()[c].svg_fill_box()).reduce(|a, b| a.union(b)),
        }
    }

    /// raea: the SVG paint properties CSS sets on this svg element, as declarations for its style attribute: a
    /// stylesheet's or a class's were left out of the serialized SVG, which usvg reads (attributes and the style
    /// attribute only), so `#b path { fill: none; stroke: … }` drew a black silhouette (#45). An inherited one is
    /// written where its computed value differs from the parent's: presentation attributes aren't in this cascade,
    /// so one still beats what the element inherits and loses to a rule on the element itself, as in Chrome.
    #[cfg(feature = "svg")]
    fn svg_css_paint(&self, current_color: Option<&str>) -> String {
        const INHERITED: [&str; 11] = [
            "fill", "fill-opacity", "fill-rule", "stroke", "stroke-width", "stroke-opacity", "stroke-linecap",
            "stroke-linejoin", "stroke-miterlimit", "stroke-dasharray", "stroke-dashoffset",
        ];
        let value = svg_computed;
        let (Some(st), Some(parent)) = (self.primary_styles(), self.parent.and_then(|p| self.tree()[p].primary_styles())) else {
            return String::new();
        };
        let mut out = Vec::new();
        for name in INHERITED {
            if let Some(v) = value(&st, name).filter(|v| value(&parent, name).as_ref() != Some(v)) {
                out.push(format!("{name}:{v}"));
            }
        }
        // not inherited: where it isn't the initial value
        if let Some(v) = value(&st, "vector-effect").filter(|v| v != "none") {
            out.push(format!("vector-effect:{v}"));
        }
        let out = out.join(";");
        match current_color {
            Some(c) => out.replace("currentcolor", c),
            None => out,
        }
    }

    /// `stroke-width` in user units, when the element is stroked: each as usvg reads the serialized element (its
    /// style attribute, then a rule's value svg_css_paint writes there, then its attribute, then what it inherits),
    /// so a stylesheet's stroke widens the stroke box too (WPT svgbox-stroke-box-001, raea #50).
    #[cfg(feature = "svg")]
    fn svg_stroke_width(&self) -> f64 {
        let Some(el) = self.element_data() else { return 0.0 };
        let style = el.attr(local_name!("style")).unwrap_or("");
        let (st, parent) = (self.primary_styles(), self.parent.and_then(|p| self.tree()[p].primary_styles()));
        let get = |name: &str| {
            let decl = style.split(';').filter_map(|d| d.split_once(':')).find(|(p, _)| p.trim() == name).map(|(_, v)| v.trim().to_string());
            let computed = st.as_ref().and_then(|st| svg_computed(st, name));
            let ruled = computed.clone().filter(|v| parent.as_ref().and_then(|p| svg_computed(p, name)).as_ref() != Some(v));
            decl.or(ruled).or_else(|| el.attr(markup5ever::LocalName::from(name)).map(str::to_string)).or(computed)
        };
        if get("stroke").is_none_or(|s| s == "none") {
            return 0.0;
        }
        get("stroke-width").and_then(|v| v.parse::<svgtypes::Length>().ok()).map_or(1.0, |l| l.number)
    }

    /// A length attribute in user units (unitless or px); None when it's missing or relative.
    #[cfg(feature = "svg")]
    fn svg_number(&self, name: &str) -> Option<f64> {
        let el = self.element_data()?;
        let value = el.attrs().iter().find(|a| &*a.name.local == name)?.value.as_str();
        let l = value.parse::<svgtypes::Length>().ok()?;
        matches!(l.unit, svgtypes::LengthUnit::None | svgtypes::LengthUnit::Px).then_some(l.number)
    }

    fn write_outer_html_in_style(&self, writer: &mut String, opts: SerializeOpts, nesting: usize) {
        const INDENT: &str = "  ";
        let style = opts.style;
        let has_children = !self.children.is_empty();
        let current_color = opts
            .resolve_current_color
            .then(|| self.primary_styles())
            .flatten()
            .map(|style| style.clone_color())
            // raea: as sRGB rgba(), which usvg reads: a computed colour's own syntax (oklch(), lab(), color()) falls
            // to black there
            .map(|color| {
                let [r, g, b, a] = *color.into_srgb_legacy().raw_components();
                let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                format!("rgba({}, {}, {}, {})", c(r), c(g), c(b), a.clamp(0.0, 1.0))
            });

        match &self.data {
            NodeData::Document(_) => {}
            NodeData::Comment { .. } => {}
            NodeData::AnonymousBlock(_) => {}
            // NodeData::Doctype { name, .. } => write!(s, "DOCTYPE {name}"),
            NodeData::Text(data) => {
                if matches!(style, OutputStyle::Pretty) {
                    for _ in 0..nesting {
                        writer.push_str(INDENT);
                    }
                }
                let in_raw_text_element = self
                    .parent
                    .and_then(|id| self.tree()[id].data.downcast_element())
                    .is_some_and(|el| {
                        // Documents are parsed with scripting disabled, so `<noscript>` is not raw text
                        el.name.ns == ns!(html)
                            && matches!(
                                el.name.local.as_ref(),
                                "style"
                                    | "script"
                                    | "xmp"
                                    | "iframe"
                                    | "noembed"
                                    | "noframes"
                                    | "plaintext"
                            )
                    });
                if in_raw_text_element {
                    writer.push_str(data.content.as_str());
                } else {
                    encode_text_to_string(data.content.as_str(), writer);
                }
                if matches!(style, OutputStyle::Pretty) {
                    writer.push('\n');
                }
            }
            NodeData::Element(data) => {
                if matches!(style, OutputStyle::Pretty) {
                    for _ in 0..nesting {
                        writer.push_str(INDENT);
                    }
                }
                writer.push('<');
                writer.push_str(&data.name.local);

                if opts.declare_xlink_ns
                    && nesting == 0
                    && !data.attrs().iter().any(|attr| is_xlink_ns_decl(&attr.name))
                {
                    writer.push_str(" xmlns:xlink=\"");
                    writer.push_str(XLINK_NS);
                    writer.push('"');
                }

                // raea: an svg element's transform as the cascade computes it (see svg_css_transform), in place of its
                // `transform` and `transform-origin` attributes; None (attributes kept) when it has no style
                #[cfg(feature = "svg")]
                let css_transform = (opts.resolve_current_color
                    && nesting > 0
                    && data.name.ns == ns!(svg)
                    && &*data.name.local != "svg")
                    .then(|| self.svg_css_transform())
                    .flatten();
                #[cfg(not(feature = "svg"))]
                let css_transform: Option<String> = None;

                // raea: the SVG paint properties CSS sets on it (a stylesheet's, a class's), into its style attribute (#45)
                #[cfg(feature = "svg")]
                let mut css_paint = (opts.resolve_current_color && data.name.ns == ns!(svg))
                    .then(|| self.svg_css_paint(current_color.as_deref()))
                    .unwrap_or_default();
                #[cfg(not(feature = "svg"))]
                let mut css_paint = String::new();
                // raea: and the computed transform there too, with its origin already in the matrix: a <style> inside
                // the svg goes to usvg with it, and usvg applies that sheet's transform and transform-origin over a
                // transform attribute (`rotate(30deg)`, which SVG's syntax can't read, so the shape drew unturned, #60).
                // A style attribute's declarations win over a sheet's in usvg, as in CSS. The identity is written too,
                // so a sheet's transform can't turn a shape the cascade leaves unturned.
                if let Some(t) = css_transform.as_deref() {
                    let t = if t.is_empty() { "matrix(1 0 0 1 0 0)" } else { t };
                    if !css_paint.is_empty() {
                        css_paint.push(';');
                    }
                    css_paint.push_str(&format!("transform:{t};transform-origin:0 0"));
                }
                for attr in data.attrs() {
                    if css_transform.is_some()
                        && attr.name.ns == ns!()
                        && matches!(&*attr.name.local, "transform" | "transform-origin")
                    {
                        continue;
                    }
                    writer.push(' ');
                    write_attr_name(&attr.name, writer);
                    writer.push_str("=\"");
                    let value_at = writer.len();
                    // raea: an svg's attribute that names var() or a colour usvg can't read, as Chrome resolves it
                    let resolved = opts
                        .resolve_current_color
                        .then(|| self.svg_attr_value(&attr.name.local, &attr.value, current_color.as_deref()))
                        .flatten();
                    // raea: usvg also reads the style attribute's transform, which the written matrix already has
                    let resolved = if css_transform.is_some() && &*attr.name.local == "style" {
                        let style = without_transform_decls(resolved.as_deref().unwrap_or(&attr.value));
                        Some(match current_color.as_deref() {
                            Some(color) => style.replace("currentColor", color),
                            None => style,
                        })
                    } else {
                        resolved
                    };
                    #[allow(clippy::unnecessary_unwrap)] // Convert to if-let chain once stabilised
                    if let Some(value) = resolved {
                        encode_quoted_attribute_to_string(&value, writer);
                    } else if current_color.is_some() && attr.value.contains("currentColor") {
                        let value = attr
                            .value
                            .replace("currentColor", current_color.as_ref().unwrap());
                        encode_quoted_attribute_to_string(&value, writer);
                    } else {
                        encode_quoted_attribute_to_string(&attr.value, writer);
                    }
                    if !css_paint.is_empty() && attr.name.ns == ns!() && &*attr.name.local == "style" {
                        // raea: after the element's own declarations when any are left, which start at the first
                        // of them. A style attribute of transforms alone is empty by now, and a `;` before the
                        // first declaration stops usvg's parser (simplecss) there: the element lost its transform
                        // and its CSS paint, so `style="transform: rotate(30deg)"` drew unturned
                        let own = from_first_declaration(&writer[value_at..]).to_string();
                        writer.truncate(value_at);
                        if !own.is_empty() {
                            writer.push_str(&own);
                            writer.push(';');
                        }
                        encode_quoted_attribute_to_string(&css_paint, writer);
                    }
                    writer.push('"');
                }
                if !css_paint.is_empty() && !data.attrs().iter().any(|a| a.name.ns == ns!() && &*a.name.local == "style") {
                    writer.push_str(" style=\"");
                    encode_quoted_attribute_to_string(&css_paint, writer);
                    writer.push('"');
                }
                if !has_children {
                    writer.push_str(" /");
                }
                writer.push('>');
                if matches!(style, OutputStyle::Pretty) {
                    writer.push('\n');
                }

                if has_children {
                    for &child_id in &self.children {
                        self.tree()[child_id].write_outer_html_in_style(writer, opts, nesting + 1);
                    }

                    if matches!(style, OutputStyle::Pretty) {
                        for _ in 0..nesting {
                            writer.push_str(INDENT);
                        }
                    }
                    writer.push_str("</");
                    writer.push_str(&data.name.local);
                    writer.push('>');
                    if matches!(style, OutputStyle::Pretty) {
                        writer.push('\n');
                    }
                }
            }
        }
    }
}

/// raea: an SVG property's computed value as CSS text (`none`, `rgb(0, 0, 0)`, `20px`), for the serialized source.
#[cfg(feature = "svg")]
fn svg_computed(st: &style::properties::ComputedValues, name: &str) -> Option<String> {
    let id = style::properties::PropertyId::parse_enabled_for_all_content(name).ok()?;
    Some(st.computed_value_to_string(id.as_shorthand().err()?))
}

/// raea: a style attribute without its transform declarations (transform, transform-origin, transform-box, translate,
/// rotate, scale), which an svg element's written `transform` matrix already resolves
fn without_transform_decls(style: &str) -> String {
    const TRANSFORM: [&str; 6] = ["transform", "transform-origin", "transform-box", "translate", "rotate", "scale"];
    style
        .split(';')
        .filter(|decl| {
            let property = decl.split_once(':').map_or("", |(p, _)| p.trim());
            !TRANSFORM.iter().any(|t| property.eq_ignore_ascii_case(t))
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// raea: a style attribute from its first declaration on: without the separators, spaces and comments before it,
/// which CSS allows there and simplecss doesn't (a comment left open runs to the end, as in CSS: nothing is left)
fn from_first_declaration(mut style: &str) -> &str {
    loop {
        style = style.trim_start_matches(|c: char| c == ';' || c.is_whitespace());
        let Some(comment) = style.strip_prefix("/*") else { return style };
        style = comment.split_once("*/").map_or("", |(_, rest)| rest);
    }
}

/// raea: a CSS colour (any syntax stylo parses) as sRGB `rgba()`, the form usvg reads; None when `value` isn't one.
fn css_color_rgba(value: &str) -> Option<String> {
    use style::parser::{Parse, ParserContext};
    use style::servo_arc::Arc as ServoArc;
    use style::stylesheets::{CssRuleType, Origin, UrlExtraData};
    let url_data = UrlExtraData(ServoArc::new(url::Url::parse("about:blank").ok()?));
    let context = ParserContext::new(
        Origin::Author,
        &url_data,
        Some(CssRuleType::Style),
        style_traits::ParsingMode::DEFAULT,
        selectors::matching::QuirksMode::NoQuirks,
        Default::default(),
        None,
        None,
        Default::default(),
    );
    let mut parser = cssparser::Parser::new(value);
    let color = parser.parse_entirely(|p| style::values::specified::Color::parse(&context, p)).ok()?;
    let color = color.to_computed_color(None).ok()?.resolve_to_absolute(&style::color::AbsoluteColor::BLACK);
    let [r, g, b, a] = *color.into_srgb_legacy().raw_components();
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(format!("rgba({}, {}, {}, {})", c(r), c(g), c(b), a.clamp(0.0, 1.0)))
}
