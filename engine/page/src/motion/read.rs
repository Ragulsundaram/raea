//! The page drawn and read back: frames, its texture, element boxes and hits.

use super::*;

impl BlitzPage {
    /// The page drawn, nothing read back: its picture is `texture`. Images and stylesheets still loading are waited
    /// for first.
    pub fn draw(&mut self) {
        for _ in 0..LOADING_UPDATES {
            self.app.update();
            if !self.loading() {
                break;
            }
            std::thread::yield_now();
        }
        self.dump();
        self.layers();
    }

    /// The page laid out and nothing drawn (`raea world`, docs/direction.feature DR1): the main world's schedules run
    /// (styles, layout) until its fonts and images are in, the render world's never, so the GPU is given no work. What
    /// `manifest` and `boxes` read after it is what `draw` would have laid out.
    pub fn lay_out(&mut self) {
        for _ in 0..LOADING_UPDATES {
            // the time the render world sends at each drawn update (bevy_render's send_time), or bevy_time warns
            if let Some(s) = self.app.get_sub_app(bevy::render::RenderApp).and_then(|r| r.world().get_resource::<bevy::time::TimeSender>()) {
                let _ = s.0.try_send(std::time::Instant::now());
            }
            self.app.main_mut().update();
            if !self.loading() {
                break;
            }
            std::thread::yield_now();
        }
    }

    /// RAEA_PAGE_DUMP leaves this page out: a page beside the composition's (raea-render's image rows, rows2d.rs) wrote
    /// its page.html over the composition's.
    pub fn not_dumped(&mut self) {
        self.dumps = false;
    }

    /// RAEA_PAGE_DUMP=DIR (sdk/react's frame dump): the document as last drawn, as DIR/page.html.
    pub(crate) fn dump(&self) {
        let Some(dir) = std::env::var_os("RAEA_PAGE_DUMP").filter(|_| self.dumps) else { return };
        let docs = self.app.world().non_send::<Docs>();
        if let Some(d) = docs.0.get(&self.page) {
            let _ = std::fs::write(std::path::Path::new(&dir).join("page.html"), d.root_element().outer_html());
        }
    }

    pub(crate) fn image(&self) -> Handle<Image> {
        self.app.world().get::<PageImage>(self.page).expect("the page's image").0.clone()
    }

    /// The page's picture as last drawn: its texture on the page's device (`Rgba8Unorm` holding sRGB-encoded
    /// premultiplied values; view it as `Rgba8UnormSrgb`).
    pub fn texture(&self) -> Option<bevy::render::render_resource::Texture> {
        self.gpu(&self.image())
    }

    /// The page drawn and read back: RGBA rows, `w` x `h`.
    pub fn frame(&mut self) -> R<Vec<u8>> {
        for _ in 0..LOADING_UPDATES {
            if !self.loading() {
                break;
            }
            self.app.update();
            std::thread::yield_now();
        }
        self.dump();
        let image = self.image();
        let px = self.read(&[(image, self.w, self.h, ENCODER_COPY)])?.remove(0);
        self.layers();
        Ok(px)
    }

    /// Each image drawn by one update and copied to the CPU once, in one submit, each into a buffer labelled `label`
    /// (tools/readbacks.sh counts copies by it: the encoder's frame is the one a render may make, B1d); rows packed.
    /// Bevy's Readback copied again on every update until its callback ran (31 copies for 3 frames).
    pub(crate) fn read(&mut self, images: &[(Handle<Image>, u32, u32, &str)]) -> R<Vec<Vec<u8>>> {
        self.app.update();
        let render = self.app.get_sub_app(bevy::render::RenderApp).ok_or("the page's render world")?;
        let device = render.world().resource::<bevy::render::renderer::RenderDevice>().wgpu_device().clone();
        let queue = render.world().resource::<bevy::render::renderer::RenderQueue>().clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("raea-page read") });
        let mut out = Vec::new();
        for (image, w, h, label) in images {
            let tex = self.gpu(image).ok_or("the page's picture isn't on the GPU yet")?;
            let row = w * 4;
            let padded = row.div_ceil(256) * 256;
            let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (padded * h) as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            encoder.copy_texture_to_buffer(
                tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(*h) } },
                wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
            );
            out.push((buf, row, padded, *h));
        }
        queue.submit([encoder.finish()]);
        for (buf, ..) in &out {
            buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        }
        device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(60)) }).map_err(|e| format!("the page's frame: {e}"))?;
        out.into_iter()
            .map(|(buf, row, padded, h)| {
                let data = buf.slice(..).get_mapped_range().map_err(|e| format!("the page's frame: {e:?}"))?;
                let mut px = Vec::with_capacity((row * h) as usize);
                for y in 0..h as usize {
                    px.extend_from_slice(&data[y * padded as usize..y * padded as usize + row as usize]);
                }
                Ok(px)
            })
            .collect()
    }

    /// Element `id`'s box as last laid out, [x, y, w, h] in page px.
    pub fn box_of(&mut self, id: u64) -> Option<[f32; 4]> {
        let docs = self.app.world().non_send::<Docs>();
        let node = docs.0.get(&self.page)?.get_node(ops::node(&self.nodes, id)?)?;
        // a text node has no layout of its own (final_layout panics on one)
        if !node.is_element() {
            return None;
        }
        let at = node.absolute_position(0.0, 0.0);
        let size = node.final_layout().size;
        Some([at.x, at.y, size.width, size.height])
    }

    /// An element's box on the page as it's drawn, transforms applied (getBoundingClientRect's), [x, y, w, h] in page px.
    pub fn client_box(&mut self, id: u64) -> Option<[f32; 4]> {
        let docs = self.app.world().non_send::<Docs>();
        let doc = docs.0.get(&self.page)?;
        let n = ops::node(&self.nodes, id)?;
        if !doc.get_node(n)?.is_element() {
            return None;
        }
        let b = doc.get_client_bounding_rect(n)?;
        Some([b.x as f32, b.y as f32, b.width as f32, b.height as f32])
    }

    /// The page's layout as a composition's script reads it (`page.layout`, docs/layout-reads.feature LR1): raea-sys-page's
    /// snapshot with offsets, each element under the id `id` gives it (None leaves it out), and `s`, the computed style
    /// of the elements in `styles` (op ids; getComputedStyle, LR5). Lay the page out first.
    pub fn layout_snapshot(&self, id: impl Fn(u64) -> Option<u64>, styles: &[u64]) -> String {
        let docs = self.app.world().non_send::<Docs>();
        let Some(doc) = docs.0.get(&self.page) else { return "{\"vw\":0,\"vh\":0,\"n\":[],\"o\":[]}".into() };
        let snap = raea_sys_page::snapshot_by(doc, &self.nodes, &id, true);
        if styles.is_empty() {
            return snap;
        }
        let mut s = serde_json::Map::new();
        for &op in styles {
            let (Some(&n), Some(key)) = (self.nodes.by_id.get(&op), id(op)) else { continue };
            let Some(cs) = doc.get_node(n).and_then(|n| n.primary_styles()) else { continue };
            s.insert(key.to_string(), computed_style(&cs));
        }
        format!("{},\"s\":{}}}", &snap[..snap.len() - 1], serde_json::Value::Object(s))
    }

    /// The element the ops made that's topmost at the point (page px), as Blitz's hit test finds it (transforms and
    /// pointer-events as drawn): the nearest element up from the hit that has an id.
    pub fn hit(&mut self, x: f32, y: f32) -> Option<u64> {
        let docs = self.app.world().non_send::<Docs>();
        let doc = docs.0.get(&self.page)?;
        let mut n = doc.hit(x, y)?.node_id;
        loop {
            if let Some(id) = self.nodes.id_of(n).filter(|id| *id != 0) {
                return Some(id);
            }
            n = doc.get_node(n)?.parent?;
        }
    }

    /// Every element the ops made from the one topmost at the point (page px) up to the root, innermost first: the
    /// path a pointer event there bubbles along.
    pub fn hit_path(&mut self, x: f32, y: f32) -> Vec<u64> {
        let docs = self.app.world().non_send::<Docs>();
        let Some(doc) = docs.0.get(&self.page) else { return Vec::new() };
        let mut out = Vec::new();
        let mut n = doc.hit(x, y).map(|h| h.node_id);
        while let Some(at) = n {
            if let Some(id) = self.nodes.id_of(at).filter(|id| *id != 0) {
                out.push(id);
            }
            n = doc.get_node(at).and_then(|d| d.parent);
        }
        out
    }

    /// Each element's box as last laid out, [x, y, w, h] in page px, by its path.
    pub fn boxes(&mut self) -> Vec<(String, [f32; 4])> {
        let docs = self.app.world().non_send::<Docs>();
        let Some(doc) = docs.0.get(&self.page) else { return vec![] };
        let mut out: Vec<(String, [f32; 4])> = self
            .tree
            .element_paths()
            .filter_map(|(id, path)| {
                let node = doc.get_node(ops::node(&self.nodes, id)?)?;
                let at = node.absolute_position(0.0, 0.0);
                let size = node.final_layout().size;
                Some((path.to_string(), [at.x, at.y, size.width, size.height]))
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

impl BlitzPage {
    /// The composition's page as layer indices at a render's frame, recorded (`raea render --layers`); a page beside
    /// the composition's (`not_dumped`) or a page with no render clock records nothing. Called once the frame's update has laid
    /// the page out (after its read, for a read frame).
    pub(crate) fn layers(&mut self) {
        let (Some((now, _, fps)), true, true) = (self.clock, self.dumps, raea_sys_layers::on()) else { return };
        let (w, h, page) = (self.w, self.h, self.page);
        let scale = self.app.world().get::<Page>(page).map_or(1.0, |p| p.scale as f64);
        let mut docs = self.app.world_mut().non_send_mut::<Docs>();
        let Some(doc) = docs.0.get_mut(&page) else { return };
        let (px, layers) = raea_sys_layers::page_ids(doc, scale, w, h);
        raea_sys_layers::with(|r| r.frame((now * fps).round() as i64, w, h, &px, &layers));
    }
}

/// What getComputedStyle answers (LR5): these properties' computed values, as stylo serializes them (Chrome's spelling:
/// px lengths, rgb() colours, a font list as written).
const COMPUTED: &[&str] = &[
    "display", "position", "visibility", "opacity", "color", "background-color", "font-family", "font-size", "font-weight", "font-style", "font-stretch",
    "line-height", "letter-spacing", "word-spacing", "text-align", "text-transform", "text-indent", "white-space", "font-feature-settings", "font-variation-settings",
    "width", "height", "top", "right", "bottom", "left", "margin-top", "margin-right", "margin-bottom", "margin-left", "padding-top", "padding-right",
    "padding-bottom", "padding-left", "border-top-width", "border-right-width", "border-bottom-width", "border-left-width", "box-sizing", "transform",
    "transform-origin", "z-index", "overflow-x", "overflow-y", "flex-direction", "justify-content", "align-items", "gap", "filter",
];

/// An element's computed style as {property: value} for [`COMPUTED`].
fn computed_style(cs: &style::properties::ComputedValues) -> serde_json::Value {
    use style::properties::{PropertyDeclarationId, PropertyId};
    let mut out = serde_json::Map::new();
    for name in COMPUTED {
        if let Some(id) = PropertyId::parse_enabled_for_all_content(name).ok().and_then(|p| p.longhand_id()) {
            out.insert((*name).into(), cs.computed_value_to_string(PropertyDeclarationId::Longhand(id)).into());
        }
    }
    serde_json::Value::Object(out)
}
