//! A camera row's place among the cameras drawing into one target (Bevy's Camera order, clear_color and viewport:
//! <Camera3d order clear_color viewport>, 3d/two_passes and 3d/split_screen).

use super::*;
use bevy::camera::Viewport;

/// The camera row's `order` among the cameras on its target, `clear` "none" to draw over the ones before without
/// clearing, and its viewport (`vp_x`, `vp_y`, `vp_w`, `vp_h`: physical px from the target's top left), on its Camera.
/// Written only when the row says something else than the Camera holds, so a camera the row doesn't move isn't
/// marked changed (the live route calls this every frame).
pub fn camera_pass(world: &mut World, e: Entity, vals: &BTreeMap<String, V>) {
    let Some(c) = world.get::<Camera>(e) else { return };
    let order = vals.get("order").map_or(c.order, |v| v.num() as isize);
    let clear = if matches!(vals.get("clear"), Some(V::S(s)) if s == "none") { ClearColorConfig::None } else { c.clear_color };
    let n = |k: &str| vals.get(k).map(|v| v.num().max(0.0) as u32);
    let viewport = match (n("vp_x"), n("vp_y"), n("vp_w"), n("vp_h")) {
        (Some(x), Some(y), Some(w), Some(h)) => Some(Viewport { physical_position: UVec2::new(x, y), physical_size: UVec2::new(w.max(1), h.max(1)), ..default() }),
        _ => c.viewport.clone(),
    };
    let same_vp = match (&viewport, &c.viewport) {
        (Some(a), Some(b)) => a.physical_position == b.physical_position && a.physical_size == b.physical_size,
        (None, None) => true,
        _ => false,
    };
    if order == c.order && same_vp && std::mem::discriminant(&clear) == std::mem::discriminant(&c.clear_color) {
        return;
    }
    let Some(mut c) = world.get_mut::<Camera>(e) else { return };
    c.order = order;
    c.clear_color = clear;
    c.viewport = viewport;
}

/// A group of the camera row's columns and what it puts on the camera (raea-sys-post camera.rs's GROUPS, and a
/// system's own given beside them, as raea-sys-fog-half's volumetric fog): `on` puts it on as the row has it (nothing,
/// or Bevy's default, where the row has none of its columns), `off` takes off what `on` put on.
pub struct Group {
    pub keys: fn(&str) -> bool,
    pub on: fn(&mut World, Entity, &BTreeMap<String, V>),
    pub off: fn(&mut World, Entity),
}

/// A group's `off` that takes the components `B` off the camera.
pub fn remove<B: Bundle>(world: &mut World, e: Entity) {
    world.entity_mut(e).remove::<B>();
}
