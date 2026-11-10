//! The target's side of a frame: the view box a tagged row draws in, supersampling (its downsample: raea-runtime-gpuframe), the render
//! world's diagnostics forwarded.

use super::*;

/// Bevy's render diagnostics (each pass's elapsed time, `render/...`), into the run's trace once a second.
#[cfg(feature = "native")]
pub(crate) fn forward_render_diagnostics(store: Res<bevy::diagnostic::DiagnosticsStore>, mut last: Local<Option<std::time::Instant>>) {
    if last.is_some_and(|t| t.elapsed().as_secs_f64() < 1.0) {
        return;
    }
    *last = Some(std::time::Instant::now());
    let passes: serde_json::Map<String, serde_json::Value> = store
        .iter()
        .filter(|d| d.path().as_str().starts_with("render/"))
        .filter_map(|d| d.smoothed().map(|v| (d.path().as_str().trim_start_matches("render/").to_string(), serde_json::json!((v * 1000.0).round() / 1000.0))))
        .collect();
    if !passes.is_empty() {
        raea_io_trace::event("bevy_passes", serde_json::json!({"ms": passes}));
    }
}

/// The Engine's target: the frame, and its tag row under it.
#[derive(Resource)]
pub(crate) struct TagRowTarget(pub(crate) Handle<Image>);

/// Engine::set_view_box's box, in target px.
#[derive(Resource, Default)]
pub(crate) struct ViewBox(pub(crate) Option<[u32; 4]>);

/// A camera drawing to the Engine's target draws a w x h Viewport of it, the frame's own, above the tag row: a view of
/// w x (h + 1) drew the 3D 0.14% larger and half a pixel low against a camera on a w x h target, a plain App's
/// (docs/blitz-bevy.feature B3 H10), and against the looks' cells. With a view box, its own view is the box's (the
/// target a sub view of it, offset by the box's place).
pub(crate) fn tag_row_view(t: Res<TagRowTarget>, images: Res<Assets<Image>>, view_box: Option<Res<ViewBox>>, mut q: Query<(&mut Camera, &RenderTarget), Without<raea_sys_frame::TagCamera>>) {
    let Some(img) = images.get(&t.0) else { return };
    let s = img.size();
    let view = match view_box.and_then(|v| v.0) {
        Some([x, y, w, h]) => bevy::camera::SubCameraView { full_size: UVec2::new(w, h), offset: Vec2::new(-(x as f32), -(y as f32)), size: s },
        None => bevy::camera::SubCameraView { full_size: UVec2::new(s.x, s.y - 1), offset: Vec2::ZERO, size: s },
    };
    for (mut c, rt) in &mut q {
        if c.viewport.is_some() { if c.sub_camera_view.is_some() { c.sub_camera_view = None; } continue; } // a camera with its own Viewport (a ViewportNode's) sees its viewport's shape, as Bevy sizes it, never the tag row
        if matches!(rt, RenderTarget::Image(i) if i.handle == t.0) && c.sub_camera_view != Some(view) {
            if view.offset == Vec2::ZERO { c.viewport = Some(bevy::camera::Viewport { physical_size: view.full_size, ..default() }) } else { c.sub_camera_view = Some(view) } // a w x h Viewport: Bevy's orthographic sub view takes its area from the w x (h + 1) target, 0.14% small (2d/sprite_slice), and a draw in clip space (a PhaseItem, custom_phase_item) maps to its h + 1 rows
        }
    }
}

/// RAEA_SS (2, 3 or 4; 1 without it): how many times the frame's size it's drawn at. Not live, and only with the
/// lossless readback, which the downsampling reads.
pub(crate) fn supersampling(live: bool) -> u32 {
    let ss = std::env::var("RAEA_SS").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).clamp(1, 4);
    if ss > 1 && (live || std::env::var("RAEA_READBACK").map(|v| v != "rgba").unwrap_or(true)) {
        eprintln!("raea: RAEA_SS needs RAEA_READBACK=rgba and a render, not a live view; drawing at 1x");
        return 1;
    }
    ss
}
