use super::*;

/// A panel whose picture changes over the run (an ImagePlane playing frames, or swapping its src): a render's lists
/// every picture it shows (`paths`; its row keys `src_k`, which one a frame shows); a live scene's has the `src` it shows
/// and its folder (`live`), and a live frame's row sends another src when it changes (raea-runtime-scene live.rs).
#[derive(Component)]
pub struct Pictures {
    pub paths: Vec<PathBuf>,
    pub shown: usize,
    pub live: Option<(String, PathBuf)>,
}

/// Each changing panel's picture for this frame: read and uploaded when `src_k` moves to another or a live row's `src`
/// names another (the one it held is dropped with its handle, so a played sequence holds one picture at a time). A card
/// whose height follows its picture (no `h`) takes the new one's shape: one cell of its sheet.
pub fn sync_pictures(frame: Res<FrameRows>, edit: Option<Res<Edit>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut q: Query<(Entity, &mut Pictures, &MeshMaterial3d<StandardMaterial>, &mut Mesh3d)>) {
    let media = edit.as_ref().map_or(&[][..], |e| &e.media[..]);
    for (e, mut pics, mat, mut mesh) in &mut q {
        let Some(v) = frame.vals(e) else { continue };
        let k = num(&v, "src_k", pics.shown as f32).round().max(0.0) as usize;
        let (src, dir) = match (&pics.live, v.get("src")) {
            (Some((shown, dir)), Some(V::S(s))) if s != shown => (s.clone(), dir.clone()),
            (None, _) if k != pics.shown => match pics.paths.get(k) {
                Some(p) => (p.to_string_lossy().to_string(), PathBuf::new()),
                None => continue,
            },
            _ => continue,
        };
        match super::panel_picture(&src, media, &dir) {
            Ok(pic) => {
                let ((pw, ph), tex) = (pic.dimensions(), images.add(super::panel_image(pic)));
                if let Some(mut m) = mats.get_mut(&mat.0) {
                    m.base_color_texture = Some(tex);
                }
                if !v.contains_key("h") {
                    let (c, r, w) = (num(&v, "sheet_cols", 1.0).max(1.0), num(&v, "sheet_rows", 1.0).max(1.0), num(&v, "w", 1.0));
                    mesh.0 = meshes.add(Rectangle::new(w, w * (ph as f32 / r) / (pw.max(1) as f32 / c)));
                }
            }
            Err(err) => eprintln!("panel picture {src}: {err}"),
        }
        pics.shown = k;
        if let Some((shown, _)) = &mut pics.live {
            *shown = src;
        }
    }
}

/// A row's entity whose values sync_meshes or sync_lights has applied: a still row isn't read again (FrameRows::still),
/// one only its placement moves has just its transform read (FrameRows::moves_only).
#[derive(Component)]
pub struct StillSynced;

/// The columns sync_meshes applies to a part's material.
const MATERIAL_COLS: [&str; 6] = ["col_r", "opacity", "emissive_k", "rough", "metal", "sheet_frame"];

/// Meshes and models where the frame's rows put them: transform, whether they show, and the material values a row
/// animates. Only columns a row has are applied, so rows written before these columns draw as they did.
#[allow(clippy::type_complexity)]
pub fn sync_meshes(
    frame: Res<FrameRows>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut glasses: ResMut<Assets<super::froxel_glass::GlassMaterial>>,
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &mut Transform,
            &mut Visibility,
            Option<&mut MeshMaterial3d<StandardMaterial>>,
            Option<(&mut super::froxel_glass::Fogged, &MeshMaterial3d<super::froxel_glass::GlassMaterial>)>,
            Has<super::plugins::Driven>,
            Option<&mut super::paint::Painted>,
            Option<&super::decal::DecalStd>,
            Option<&raea_sys_shader::Shaded>,
            Has<super::SharedMaterial>,
            Has<StillSynced>,
        ),
        Or<(With<Mesh3d>, With<Model3d>, With<Instanced>)>,
    >,
) {
    let place = frame.placer();
    for (e, mut t, mut vis, mut mat, mut fogged, driven, mut painted, decal, shaded, shared, synced) in &mut q {
        // a row the same on every frame is applied once (10 000 still cubes made their values every frame); a live
        // scene's rows are never still
        let still = frame.still(e);
        if still && synced {
            continue;
        }
        // a row only its placement moves, its other values applied once: its transform alone (FrameRows::moves_only)
        let placed = !still && frame.moves_only(e, &raea_sys_frame::PLACE);
        if placed && synced {
            let Some(nt) = place(e) else { continue };
            if *t != nt && !driven {
                *t = nt;
            }
            continue;
        }
        let Some(v) = frame.vals(e) else { continue };
        let v = &v;
        // a mesh a Bevy plugin moves (a rigid body) keeps the plugin's transform (engine/plugins.rs)
        let nt = transform_of(v);
        if *t != nt && !driven {
            *t = nt;
        }
        if let Some(on) = v.get("visible") {
            let want = if on.truthy() { Visibility::Inherited } else { Visibility::Hidden };
            if *vis != want {
                *vis = want;
            }
        }
        // a painted mesh's standard material is kept aside for this (paint.rs), and a see-through mesh drawn with its froxel
        // fog copy (froxel_glass.rs) follows its standard material
        // (and a decal's, decal.rs, and a shaded one's, raea-sys-shader)
        let Some(h) = mat.as_ref().map(|m| m.0.clone()).or(painted.as_ref().map(|p| p.std.clone())).or(fogged.as_ref().map(|(f, _)| f.std.clone())).or(decal.map(|d| d.0.clone())).or(shaded.map(|s| s.std.clone())) else {
            // no material yet: a still row with material columns waits for one (a painted or shaded copy comes later)
            if (still || placed) && !MATERIAL_COLS.iter().any(|k| v.contains_key(*k)) {
                commands.entity(e).insert(StillSynced);
            }
            continue;
        };
        if still || placed {
            commands.entity(e).insert(StillSynced);
        }
        let h = &h;
        // a sprite sheet's cell (an ImagePlane's `frame` of its `grid`)
        if let Some(fr) = v.get("sheet_frame") {
            let want = sheet_uv(fr.num() as f32, num(v, "sheet_cols", 1.0), num(v, "sheet_rows", 1.0));
            if mats.get(h).is_some_and(|m| m.uv_transform != want) {
                if let Some(mut m) = mats.get_mut(h) {
                    m.uv_transform = want;
                }
            }
        }
        let animated = ["col_r", "opacity", "emissive_k", "rough", "metal"].iter().any(|k| v.contains_key(*k));
        if !animated {
            continue;
        }
        let Some(cur) = mats.get(h) else { continue };
        let mut base = cur.base_color;
        if v.contains_key("col_r") {
            base = Color::srgb(num(v, "col_r", 0.8), num(v, "col_g", 0.8), num(v, "col_b", 0.8));
        }
        if v.contains_key("opacity") {
            base.set_alpha(num(v, "opacity", 1.0).clamp(0.0, 1.0));
        } else {
            base.set_alpha(cur.base_color.alpha());
        }
        let rough = if v.contains_key("rough") { num(v, "rough", 0.45) } else { cur.perceptual_roughness };
        let metal = if v.contains_key("metal") { num(v, "metal", 0.0) } else { cur.metallic };
        let emissive = if v.contains_key("emissive_k") {
            let em = colour(v.get("emissive"), [0.0, 0.0, 0.0, 1.0]).to_linear() * num(v, "emissive_k", 1.0);
            LinearRgba::rgb(em.red, em.green, em.blue)
        } else {
            cur.emissive
        };
        // a mesh with an opacity column blends below 1 and is opaque at 1 (writing depth), each frame, render and live alike
        // (#38); one whose alpha mode is given, or a blended picture (a glyph: its see-through parts), stays as it is at 1
        let want = if base.alpha() < 1.0 || cur.base_color_texture.is_some() && cur.alpha_mode == AlphaMode::Blend { AlphaMode::Blend } else { AlphaMode::Opaque };
        let blend = v.contains_key("opacity") && !v.contains_key("alpha") && matches!(cur.alpha_mode, AlphaMode::Opaque | AlphaMode::Blend) && cur.alpha_mode != want;
        if cur.base_color != base || cur.perceptual_roughness != rough || cur.metallic != metal || cur.emissive != emissive || blend {
            // a material other parts hold too (share.rs): this part's change goes on a copy of its own, put where its
            // handle is kept; a fog copy follows the new handle a frame later (froxel_glass.rs)
            if shared {
                let mut own = cur.clone();
                own.base_color = base;
                own.perceptual_roughness = rough;
                own.metallic = metal;
                own.emissive = emissive;
                if blend {
                    own.alpha_mode = want;
                }
                let nh = mats.add(own);
                if let Some(m) = mat.as_mut() {
                    m.0 = nh;
                } else if let Some(p) = painted.as_mut() {
                    p.std = nh;
                } else if let Some((f, _)) = fogged.as_mut() {
                    f.std = nh;
                }
                commands.entity(e).remove::<super::SharedMaterial>();
                continue;
            }
            if let Some(mut m) = mats.get_mut(h) {
                m.base_color = base;
                m.perceptual_roughness = rough;
                m.metallic = metal;
                m.emissive = emissive;
                if blend {
                    m.alpha_mode = want;
                }
                // and its fog copy, this frame (froxel_glass.rs's own follow is a frame behind)
                if let Some(mut g) = fogged.as_ref().and_then(|(_, g)| glasses.get_mut(&g.0)) {
                    g.base = m.clone();
                }
            }
        }
    }
}

/// The camera where the frame's row puts it, looking where it says; its field of view and ambient light.
///
/// `placed` is where each camera's row last put it (position, look-at). The Transform is written only when the row's
/// differs, so a static row leaves the camera to a controller (bevy_panorbit_camera, bevy_pancam) after placing it
/// once, and a row that moves (or moves back) writes every frame it changes. An entity with no entry always writes,
/// so a render seeking straight to frame N places its camera.
#[allow(clippy::type_complexity)]
pub fn sync_camera(frame: Res<FrameRows>, edit: Option<Res<Edit>>, mut placed: Local<bevy::platform::collections::HashMap<Entity, (Vec3, Vec3, Option<Quat>)>>, mut q: Query<(Entity, &mut Transform, Option<&mut Projection>, Option<&mut AmbientLight>, Option<&mut bevy::post_process::dof::DepthOfField>, Option<&mut super::lens::Lens>, Option<&mut bevy::post_process::effect_stack::ChromaticAberration>, Option<&mut Camera>), (With<Camera3d>, Without<super::BareEntity>)>) {
    for (e, mut t, proj, amb, dof, lens, ab, cam) in &mut q {
        let Some(v) = frame.vals(e) else { continue };
        let v = &v;
        let pos = Vec3::new(num(v, "x", 0.0), num(v, "y", 2.0), num(v, "z", 9.0));
        let look = Vec3::new(num(v, "look_x", 0.0), num(v, "look_y", 1.0), num(v, "look_z", 0.0));
        // a Camera3d's own rotation (rot_qx..qw), exactly as its Transform holds it: its look-at point rebuilt into a
        // rotation in f32 is a millionth of a radian off, which a raymarch through the view's matrices shows
        let rot = row_rotation(v);
        if (rot.is_some() || (look - pos).length_squared() > 1e-12) && placed.get(&e) != Some(&(pos, look, rot)) {
            placed.insert(e, (pos, look, rot));
            let nt = match rot {
                Some(r) => Transform::from_translation(pos).with_rotation(r),
                None => Transform::from_translation(pos).looking_at(look, up_for(look - pos)),
            };
            if *t != nt {
                *t = nt;
            }
        }
        let mut seen = None;
        if let Some(mut p) = proj {
            // an orthographic camera (`ortho`: the view's height in metres, keyed; 0 is a perspective one again)
            if let Some(h) = v.get("ortho").map(|h| h.num() as f32) {
                if h > 0.0 {
                    if let Projection::Orthographic(o) = p.as_mut() {
                        match &mut o.scaling_mode {
                            bevy::camera::ScalingMode::FixedVertical { viewport_height } if *viewport_height == h => {}
                            m => *m = bevy::camera::ScalingMode::FixedVertical { viewport_height: h },
                        }
                    } else {
                        let mut o = bevy::camera::OrthographicProjection::default_3d();
                        o.scaling_mode = bevy::camera::ScalingMode::FixedVertical { viewport_height: h };
                        if let Some(far) = v.get("far") {
                            o.far = far.num() as f32;
                        }
                        *p = Projection::Orthographic(o);
                    }
                } else if !matches!(*p, Projection::Perspective(_)) {
                    *p = Projection::Perspective(bevy::camera::PerspectiveProjection { near: num(v, "near", 0.1), far: num(v, "far", 1000.0), ..default() });
                }
            }
            if let Projection::Perspective(pp) = p.as_mut() {
                if let Some(fov) = v.get("fov") {
                    // a fisheye with its own fov widens the render by its strength (lens.rs)
                    let f = super::lens::render_fov(v, (fov.num() as f32).to_radians());
                    if pp.fov != f {
                        pp.fov = f;
                    }
                }
                // the target's own shape (the projection's aspect is only updated after this, in PostUpdate)
                let aspect = edit.as_ref().map(|e| e.prog.width as f32 / e.prog.height.max(1) as f32).unwrap_or(pp.aspect_ratio);
                seen = Some((pp.fov, aspect));
            }
        }
        if let (Some(mut l), Some((fov, aspect))) = (lens, seen) {
            let want = super::lens::lens_of(v, fov, aspect);
            if *l != want {
                *l = want;
            }
        }
        if let Some(mut dof) = dof {
            if let Some(d) = v.get("focus").map(|d| d.num() as f32).filter(|d| dof.focal_distance != *d) {
                dof.focal_distance = d;
            }
            // a frame without a <DepthOfField> keys its aperture shut (runtime/look NO_DOF)
            if let Some(a) = v.get("dof_aperture").map(|a| a.num() as f32).filter(|a| dof.aperture_f_stops != *a) {
                dof.aperture_f_stops = a;
            }
        }
        // a <Grade>'s fringes, keyed: this frame's own, never one left by an earlier frame
        if let Some(mut c) = ab {
            let a = num(v, "aberration", 0.0);
            if c.intensity != a {
                c.intensity = a;
            }
        }
        if let (Some(b), Some(mut a)) = (v.get("ambient"), amb) {
            if a.brightness != b.num() as f32 {
                a.brightness = b.num() as f32;
            }
        }
        // a clear keyed frame by frame (bg_r..bg_a, sRGB: a holder's background that changes, runtime/export program.rs)
        if let (Some(r), Some(mut c)) = (v.get("bg_r"), cam) {
            let want = Color::srgba(r.num() as f32, num(v, "bg_g", 0.0), num(v, "bg_b", 0.0), num(v, "bg_a", 1.0));
            if !matches!(c.clear_color, ClearColorConfig::Custom(have) if have == want) {
                c.clear_color = ClearColorConfig::Custom(want);
            }
        }
    }
}

/// Lights where the frame's rows put them, as bright as they say.
#[allow(clippy::type_complexity)]
pub fn sync_lights(frame: Res<FrameRows>, mut commands: Commands, mut q: Query<(Entity, &mut Transform, Option<&mut DirectionalLight>, Option<&mut PointLight>, Option<&mut SpotLight>, Option<&CastsShadows>, Option<&mut bevy::light::RectLight>, Has<StillSynced>, Option<&mut bevy::light::CascadeShadowConfig>)>) {
    // a dark light (a frame it isn't in: its power is 0) has its shadow maps off, so no shadow pass is drawn for it. Lit or dark,
    // every light stays shown: hiding the dark ones (so Bevy doesn't extract them) put the lit volumetric spots' shafts
    // out, as Bevy 0.19's prepare_lights indexes its spot lights past a count of point lights that the hidden ones
    // threw off. Dark with its maps on, a film's every shadow-casting light drew them on every frame of every scene
    // (the rome film: 20 lights, 4096-texel maps; 2026-10-20)
    for (e, mut t, d, p, s, casts, rect, synced, config) in &mut q {
        if d.is_none() && p.is_none() && s.is_none() && rect.is_none() {
            continue;
        }
        // a light the same on every frame is applied once, as a still mesh is (100 000 lights made their values each frame)
        let still = frame.still(e);
        if still && synced {
            continue;
        }
        let Some(v) = frame.vals(e) else { continue };
        if still {
            commands.entity(e).insert(StillSynced);
        }
        let v = &v;
        let pos = Vec3::new(num(v, "x", 0.0), num(v, "y", 0.0), num(v, "z", 0.0));
        let dir = Vec3::new(num(v, "dir_x", -0.5), num(v, "dir_y", -1.0), num(v, "dir_z", -0.4));
        // a Bevy light's own rotation (rot_qx..qw), exactly as its Transform holds it, over its direction turned back
        let rot = row_rotation(v);
        let nt = if p.is_some() {
            Transform::from_translation(pos)
        } else if let Some(r) = rot {
            Transform::from_translation(if d.is_some() { Vec3::ZERO } else { pos }).with_rotation(r)
        } else if s.is_some() || rect.is_some() {
            Transform::from_translation(pos).looking_to(dir, up_for(dir))
        } else {
            Transform::default().looking_to(dir, up_for(dir))
        };
        if *t != nt {
            *t = nt;
        }
        let shadows = |power: f32, now: bool| casts.map_or(now, |c| c.0 && power > 0.0);
        if let Some(mut d) = d {
            let lux = num(v, "lux", 8000.0);
            if d.illuminance != lux {
                d.illuminance = lux;
            }
            let sh = shadows(lux, d.shadow_maps_enabled);
            if d.shadow_maps_enabled != sh {
                d.shadow_maps_enabled = sh;
            }
            // the frame's <Shadows>: its cascades and biases (RR1)
            let (mut b, mut nb) = (d.shadow_depth_bias, d.shadow_normal_bias);
            if biases(v, &mut b, &mut nb) { (d.shadow_depth_bias, d.shadow_normal_bias) = (b, nb); }
            let c = cascades(v);
            if let Some(mut was) = config.filter(|was| was.bounds != c.bounds || was.overlap_proportion != c.overlap_proportion) { *was = c; }
        }
        if let Some(mut p) = p {
            let lm = num(v, "lumens", 1.0e6);
            if p.intensity != lm {
                p.intensity = lm;
            }
            let sh = shadows(lm, p.shadow_maps_enabled);
            if p.shadow_maps_enabled != sh {
                p.shadow_maps_enabled = sh;
            }
            let (mut b, mut nb) = (p.shadow_depth_bias, p.shadow_normal_bias);
            if biases(v, &mut b, &mut nb) { (p.shadow_depth_bias, p.shadow_normal_bias) = (b, nb); }
        }
        if let Some(mut r) = rect {
            let lm = num(v, "lumens", 1.0e6);
            if r.intensity != lm {
                r.intensity = lm;
            }
        }
        if let Some(mut s) = s {
            let lm = num(v, "lumens", 1.0e6);
            if s.intensity != lm {
                s.intensity = lm;
            }
            // its cone, as its rows animate it (spotlight.rs's light_sway widens every light each frame; light.rs set it once)
            let outer = num(v, "outer", std::f32::consts::FRAC_PI_4);
            let inner = num(v, "inner", 0.0).min(outer);
            if s.outer_angle != outer || s.inner_angle != inner { (s.outer_angle, s.inner_angle) = (outer, inner); }
            let sh = shadows(lm, s.shadow_maps_enabled);
            if s.shadow_maps_enabled != sh {
                s.shadow_maps_enabled = sh;
            }
            let (mut b, mut nb) = (s.shadow_depth_bias, s.shadow_normal_bias);
            if biases(v, &mut b, &mut nb) { (s.shadow_depth_bias, s.shadow_normal_bias) = (b, nb); }
        }
    }
}

/// A model attached to another's bone (attach_to: the other's row id, attach_bone: the bone's node name): once both
/// are ready, made a child of the bone, so it follows the bone's animated pose and its own placement is the bone's
/// local space (a hat on a walking man's head). Set once.
#[derive(Component)]
pub struct Attached;

pub fn sync_attach(
    mut commands: Commands,
    frame: Res<FrameRows>,
    models: Query<(Entity, &Model3d, Has<Attached>)>,
    names: Query<&Name>,
    kids: Query<&Children>,
    mut err: ResMut<Error>,
) {
    for (e, m, done) in &models {
        if done || !matches!(m.state, ModelState::Ready { .. }) {
            continue;
        }
        let Some(v) = frame.vals(e) else { continue };
        let Some(V::S(to)) = v.get("attach_to") else { continue };
        let Some((target, tm, _)) = models.iter().find(|(_, o, _)| &o.id == to) else {
            err.0 = Some(format!("model {}: attached to {to}, which isn't a model", m.src));
            continue;
        };
        if !matches!(tm.state, ModelState::Ready { .. }) {
            continue;
        }
        let bone = match v.get("attach_bone") { Some(V::S(b)) => b.clone(), _ => String::new() };
        match descendants(target, &kids).into_iter().find(|k| names.get(*k).is_ok_and(|n| n.as_str() == bone)) {
            Some(b) => {
                commands.entity(e).insert((ChildOf(b), Attached));
            }
            None => err.0 = Some(format!("model {}: {} has no node named {bone:?}", m.src, tm.src)),
        }
    }
}

/// A skinned model's joints, found once it's loaded: each joint's entity and the row whose placement it takes.
#[derive(Component)]
pub struct Bones(Vec<(Entity, Entity)>);

/// A skinned model's joints placed where other rows are (bone_joints, bone_rows: <Model bones={{ joint: name }}>): each
/// frame each joint takes its row's placement (translation and rotation; the row can be hidden), so a ragdoll's bodies
/// move one skin that bends at the joints. Its meshes aren't culled by their bind pose's bounds, which the skin leaves
/// behind as the joints travel.
pub fn sync_bones(
    mut commands: Commands,
    frame: Res<FrameRows>,
    models: Query<(Entity, &Model3d, Option<&Bones>)>,
    names: Query<&Name>,
    kids: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
    mut tf: Query<&mut Transform>,
    mut err: ResMut<Error>,
) {
    for (e, m, bones) in &models {
        if !matches!(m.state, ModelState::Ready { .. }) {
            continue;
        }
        let pairs = match bones {
            Some(b) => b.0.clone(),
            None => {
                let Some(v) = frame.vals(e) else { continue };
                let (Some(V::S(joints)), Some(V::S(rows))) = (v.get("bone_joints"), v.get("bone_rows")) else { continue };
                let tree = descendants(e, &kids);
                let mut pairs = Vec::new();
                for (joint, row) in joints.split(',').zip(rows.split(',')).filter(|(j, _)| !j.is_empty()) {
                    let Some(je) = tree.iter().copied().find(|k| names.get(*k).is_ok_and(|n| n.as_str() == joint)) else {
                        err.0 = Some(format!("model {}: has no joint named {joint:?}", m.src));
                        continue;
                    };
                    let Some((re, _, _)) = models.iter().find(|(_, o, _)| o.id == row) else {
                        err.0 = Some(format!("model {}: joint {joint} follows {row}, which isn't a model", m.src));
                        continue;
                    };
                    pairs.push((je, re));
                }
                for k in tree.into_iter().filter(|k| meshes.contains(*k)) {
                    commands.entity(k).insert(bevy::camera::visibility::NoFrustumCulling);
                }
                commands.entity(e).insert(Bones(pairs.clone()));
                pairs
            }
        };
        for (joint, row) in pairs {
            let Ok(at) = tf.get(row).copied() else { continue };
            if let Ok(mut t) = tf.get_mut(joint) {
                if t.translation != at.translation || t.rotation != at.rotation {
                    t.translation = at.translation;
                    t.rotation = at.rotation;
                    t.scale = Vec3::ONE;
                }
            }
        }
    }
}

/// A row's own rotation (rot_qx..qw: a Camera3d's or a Bevy light's Transform rotation, exact), normalized only between
/// keyed frames, where its components were interpolated.
fn row_rotation(v: &BTreeMap<String, V>) -> Option<Quat> {
    let r = v.contains_key("rot_qw").then(|| Quat::from_xyzw(num(v, "rot_qx", 0.0), num(v, "rot_qy", 0.0), num(v, "rot_qz", 0.0), num(v, "rot_qw", 1.0)))?;
    Some(if r.is_normalized() { r } else { r.normalize() })
}
