use super::*;
pub use raea_sys_fog_half::camera::{FogDither, VFOG, fog_reference, sync_fog_dither};

/// Each camera's post-processing as this frame's row has it (raea-sys-post camera.rs), volumetric fog's group with it.
pub fn sync_post(world: &mut World) {
    raea_sys_post::camera::sync(world, &[VFOG]);
}

/// A sprite sheet's cell `frame` of `cols` x `rows` (0 the top left, read along each row) as a card's UV transform.
pub fn sheet_uv(frame: f32, cols: f32, rows: f32) -> bevy::math::Affine2 {
    let (c, r) = (cols.max(1.0), rows.max(1.0));
    let k = frame.max(0.0).floor() % (c * r);
    let (x, y) = (k % c, (k / c).floor());
    bevy::math::Affine2::from_scale_angle_translation(Vec2::new(1.0 / c, 1.0 / r), 0.0, Vec2::new(x / c, y / r))
}

/// An up vector that isn't along `dir` (a light pointing straight down).
pub fn up_for(dir: Vec3) -> Vec3 {
    if dir.normalize_or_zero().cross(Vec3::Y).length_squared() < 1e-6 { Vec3::Z } else { Vec3::Y }
}

/// The camera row's layers, order, clear and projection, and its post-processing (raea-sys-post camera.rs).
pub fn camera(world: &mut World, e: Entity, vals: &BTreeMap<String, V>) {
    let g = |k: &str, d: f32| num(vals, k, d);
    layers(world, e, vals);
    // its order, clear and viewport among the cameras on its target (<Camera3d order clear_color viewport>)
    raea_sys_frame::camera::camera_pass(world, e, vals);
    if vals.contains_key("near") || vals.contains_key("far") {
        let fov = g("fov", 45.0).to_radians();
        world.entity_mut(e).insert(Projection::Perspective(PerspectiveProjection { fov, near: g("near", 0.1), far: g("far", 1000.0), ..default() }));
    }
    // the post-processing, put on again on any frame whose row changes it (raea-sys-post camera.rs, RR1)
    raea_sys_post::camera::apply(world, e, vals, &[VFOG]);
}

/// raea's volumetric fog shader (volumetric_fog.wesl: Bevy 0.20.0's, its dither seeded from the film frame) over
/// Bevy's embedded one. The fog pipeline loads its shader when it is first made, after the plugins are built, so the
/// copy registered here is the one it gets.
pub fn install_fog_shader(app: &mut App) {
    let swapped = super::bevy_api::swap_embedded_shader(app, "bevy_pbr/volumetric_fog/volumetric_fog.wesl", 0xd6b9_2688_4641_46be, |_| {
        let mut s = include_str!("volumetric_fog.wesl").to_string();
        // the reference marches the whole ray (Bevy's stops short off the view axis) and scatters the ambient light
        // by density, as the grid does (RAEA_FOG_REFERENCE_AMBIENT=flat: Bevy's flat ambient)
        if fog_reference() {
            s = replace_once(&s, "const RAY_LENGTH: bool = false;", "const RAY_LENGTH: bool = true;");
            if std::env::var("RAEA_FOG_REFERENCE_AMBIENT").as_deref() != Ok("flat") {
                s = replace_once(&s, "const SCATTERED_AMBIENT: bool = false;", "const SCATTERED_AMBIENT: bool = true;");
            }
        }
        s
    });
    assert!(swapped, "raea: Bevy no longer embeds bevy_pbr/volumetric_fog/volumetric_fog.wesl (docs/bevy-upgrade.md)");
}
