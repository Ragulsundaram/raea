//! A material written in TypeScript (docs/bevy-ts.feature G20): `<ShaderMaterial>` in raea/3d, or a
//! MeshMaterial3d<ShaderMaterial> whose handle is given inline. One Rust type covers every such material, the way
//! Bevy's own materials are typed: a fixed bind group (AsBindGroup) of 16 vec4 uniform slots and four optional
//! textures, and a fragment shader chosen per material in `specialize` from its bind-group data (a uuid shader handle
//! made from the source's hash), so two materials with different WGSL are two pipelines of the same type.
//!
//! The source the TypeScript gives (`wesl`, or `wgsl`, which is the same thing: WESL is a superset of WGSL) is wrapped
//! in a WESL module raea-render writes, since Bevy 0.20 compiles shader imports with WESL (naga_oil and its `#import` are gone):
//!   `import bevy_pbr::render::forward_io::VertexOutput;` and the mesh view bindings (globals.time)
//!   the uniform slots in `@group(constants::MATERIAL_BIND_GROUP)`, with an accessor for each named uniform: `u_tint()`
//!   (vec4), `u_speed()` (f32)...
//!   each texture and its sampler as `tex_NAME` and `samp_NAME`
//!   the source itself, and, when it has no `@fragment`, an entry point calling its `fn shade(in: VertexOutput)
//!   -> vec4<f32>`
//! A uniform is a number (f32), [x, y] (vec2), [x, y, z] (vec3), [x, y, z, w] (vec4) or "#rrggbb(aa)" (vec4, linear).

use super::*;
use bevy::pbr::{Material, MaterialPlugin};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::{ShaderDefVal, ShaderRef};
use serde_json::Value;
use std::hash::{Hash, Hasher};

pub fn install(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ShaderMaterial>::default());
    app.register_type::<bevy::pbr::MeshMaterial3d<ShaderMaterial>>();
}

/// Slots a material's uniforms pack into, one vec4 each.
pub const SLOTS: usize = 16;
/// Textures a material may name.
pub const TEXTURES: usize = 4;

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ShaderKey)]
pub struct ShaderMaterial {
    #[uniform(0)]
    slots: [Vec4; SLOTS],
    #[texture(1)]
    #[sampler(2)]
    tex0: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    tex1: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    tex2: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    tex3: Option<Handle<Image>>,
    /// the one texture array (docs/shaders.feature SH9)
    #[texture(9, dimension = "2d_array")]
    #[sampler(10)]
    arr: Option<Handle<Image>>,
    shader: u128,
    alpha: AlphaMode,
    double_sided: bool,
}

/// What picks a material's pipeline: its shader (a uuid handle) and how it blends and culls.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShaderKey {
    shader: u128,
    double_sided: bool,
}

impl From<&ShaderMaterial> for ShaderKey {
    fn from(m: &ShaderMaterial) -> Self {
        ShaderKey { shader: m.shader, double_sided: m.double_sided }
    }
}

impl Material for ShaderMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // (the shader is the colour pass's only: a prepass keeps Bevy's own fragment, which writes the normals and motion
        // a prepass target expects. Under Solari the camera has a normal and motion prepass, and the colour shader in it
        // failed validation, "Error matching ShaderStages(FRAGMENT) ... label = 'prepass_pipeline'": Stay's fern drops)
        let prepass = descriptor.label.as_deref().is_some_and(|l| l.contains("prepass"));
        if let (false, Some(f)) = (prepass, descriptor.fragment.as_mut()) {
            f.shader = Handle::Uuid(bevy::asset::uuid::Uuid::from_u128(key.bind_group_data.shader), core::marker::PhantomData);
        }
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// One named uniform: its slot and WGSL type.
pub struct Uniform {
    pub name: String,
    pub ty: &'static str,
    pub value: Vec4,
}

pub fn ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_') && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

/// A spec's `defs`, {NAME: true | false | a whole number} (docs/shaders.feature SH8): each a WESL condition for
/// `@if(NAME)` (a number is true), and a number also `constants::NAME`. They go on the module's Shader and in its
/// hash, so each set of defs is its own pipeline.
pub fn defs(o: &serde_json::Map<String, Value>, what: &str) -> Result<Vec<ShaderDefVal>, String> {
    let Some(d) = o.get("defs") else { return Ok(vec![]) };
    let d = d.as_object().ok_or_else(|| format!("{what}'s defs are {{NAME: true | false | a whole number}}"))?;
    let mut out = vec![];
    for (k, v) in d {
        if !ident(k) {
            return Err(format!("a def's name is a WGSL identifier, got {k:?}"));
        }
        out.push(match v {
            Value::Bool(b) => ShaderDefVal::Bool(k.clone().into(), *b),
            _ => match v.as_i64().and_then(|n| i32::try_from(n).ok()) {
                Some(n) => ShaderDefVal::Int(k.clone().into(), n),
                None => return Err(format!("{what}: def {k} is true, false or a whole number, got {v}")),
            },
        });
    }
    Ok(out)
}

pub fn uniform(name: &str, v: &Value) -> Result<Uniform, String> {
    if !ident(name) {
        return Err(format!("a uniform's name is a WGSL identifier, got {name:?}"));
    }
    let (ty, value) = match v {
        Value::Number(n) => ("f32", Vec4::new(n.as_f64().unwrap_or(0.0) as f32, 0.0, 0.0, 0.0)),
        Value::String(s) => {
            let c = Srgba::hex(s.trim_start_matches('#')).map_err(|e| format!("uniform {name}: {s:?} isn't a colour: {e}"))?;
            let l = LinearRgba::from(c);
            ("vec4<f32>", Vec4::new(l.red, l.green, l.blue, l.alpha))
        }
        Value::Array(a) if (2..=4).contains(&a.len()) => {
            let f: Vec<f32> = a.iter().map(|x| x.as_f64().map(|f| f as f32).ok_or_else(|| format!("uniform {name}: numbers, got {x}"))).collect::<Result<_, _>>()?;
            let ty = ["vec2<f32>", "vec3<f32>", "vec4<f32>"][a.len() - 2];
            (ty, Vec4::new(f[0], f[1], *f.get(2).unwrap_or(&0.0), *f.get(3).unwrap_or(&0.0)))
        }
        _ => return Err(format!("uniform {name} is a number, [x, y], [x, y, z], [x, y, z, w] or \"#rrggbb\", got {v}")),
    };
    Ok(Uniform { name: name.to_string(), ty, value })
}

/// The module a material's source is compiled as, in WESL: the imports, its uniforms' and textures' bindings and
/// accessors, the source, and an entry point unless it has its own. Bevy 0.20 dropped naga_oil, so a WGSL source has
/// no `#import` of its own any more; WESL is a superset of WGSL, so a `wgsl` source goes in this module unchanged.
pub fn module(source: &str, uniforms: &[(String, &'static str)], textures: &[String], array: Option<&str>) -> String {
    let group = "constants::MATERIAL_BIND_GROUP";
    let mut m = String::from("import bevy_pbr::render::forward_io::VertexOutput;\nimport bevy_pbr::render::mesh_view_bindings::{view, globals};\n\n");
    m += &format!("struct RaeaSlots {{ s: array<vec4<f32>, 16> }};\n@group({group}) @binding(0) var<uniform> raea_slots: RaeaSlots;\n");
    for (i, t) in textures.iter().enumerate() {
        m += &format!(
            "@group({group}) @binding({}) var tex_{t}: texture_2d<f32>;\n@group({group}) @binding({}) var samp_{t}: sampler;\n",
            1 + i * 2,
            2 + i * 2
        );
    }
    if let Some(a) = array {
        m += &format!("@group({group}) @binding(9) var tex_{a}: texture_2d_array<f32>;\n@group({group}) @binding(10) var samp_{a}: sampler;\n");
    }
    for (i, (n, ty)) in uniforms.iter().enumerate() {
        let body = match *ty {
            "f32" => format!("raea_slots.s[{i}].x"),
            "vec2<f32>" => format!("raea_slots.s[{i}].xy"),
            "vec3<f32>" => format!("raea_slots.s[{i}].xyz"),
            _ => format!("raea_slots.s[{i}]"),
        };
        m += &format!("fn u_{n}() -> {ty} {{ return {body}; }}\n");
    }
    m += "\n";
    m += source;
    if !source.contains("@fragment") {
        m += "\n\n@fragment\nfn fragment(in: VertexOutput) -> @location(0) vec4<f32> {\n    return shade(in);\n}\n";
    }
    m
}

/// A ShaderMaterial from what a Handle<ShaderMaterial> field gives (Fields::handle): {wgsl, uniforms, textures, alpha,
/// double_sided}, its shader added to Assets<Shader> under the uuid of its module's hash. `image` resolves a texture
/// (a path, an inline picture, a "render:NAME" image) as any Handle<Image> field.
pub fn build(world: &mut World, v: &Value, mut image: impl FnMut(&mut World, &Value) -> Result<Handle<Image>, String>) -> Result<ShaderMaterial, String> {
    let o = v.as_object().ok_or_else(|| format!("a ShaderMaterial is {{wgsl, uniforms, textures, alpha}}, got {v}"))?;
    let src = match (o.get("wgsl").and_then(Value::as_str), o.get("wesl").and_then(Value::as_str)) {
        (Some(_), Some(_)) => return Err("a ShaderMaterial takes wgsl or wesl, not both".into()),
        (Some(s), None) | (None, Some(s)) => s,
        (None, None) => return Err("a ShaderMaterial needs wesl or wgsl: its fragment source (a `fn shade(in: VertexOutput) -> vec4<f32>`, or its own @fragment)".into()),
    };
    let mut us = vec![];
    if let Some(u) = o.get("uniforms") {
        let u = u.as_object().ok_or("a ShaderMaterial's uniforms are {name: value}")?;
        for (k, x) in u {
            us.push(uniform(k, x)?);
        }
    }
    if us.len() > SLOTS {
        return Err(format!("a ShaderMaterial has at most {SLOTS} uniforms, got {}", us.len()));
    }
    let mut texs: Vec<(String, Handle<Image>)> = vec![];
    let mut arr: Option<(String, Handle<Image>)> = None;
    if let Some(t) = o.get("textures") {
        let t = t.as_object().ok_or("a ShaderMaterial's textures are {name: picture}")?;
        for (k, x) in t {
            if !ident(k) {
                return Err(format!("a texture's name is a WGSL identifier, got {k:?}"));
            }
            if raea_sys_sampler::arrays::is_array(x) {
                if arr.is_some() {
                    return Err(format!("a ShaderMaterial has one texture array, got a second ({k})"));
                }
                arr = Some((k.clone(), raea_sys_sampler::arrays::array(world, x, &format!("ShaderMaterial texture {k}"), &mut image)?));
                continue;
            }
            texs.push((k.clone(), raea_sys_sampler::texture(world, x, &format!("ShaderMaterial texture {k}"), &mut image).map_err(|e| format!("texture {k}: {e}"))?));
        }
    }
    if texs.len() > TEXTURES {
        return Err(format!("a ShaderMaterial has at most {TEXTURES} textures, got {}", texs.len()));
    }
    let names: Vec<(String, &'static str)> = us.iter().map(|u| (u.name.clone(), u.ty)).collect();
    let tnames: Vec<String> = texs.iter().map(|t| t.0.clone()).collect();
    let module = module(src, &names, &tnames, arr.as_ref().map(|a| a.0.as_str()));
    let defs = defs(o, "a ShaderMaterial")?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    module.hash(&mut h);
    defs.hash(&mut h);
    let hi = h.finish();
    "raea shader material".hash(&mut h);
    let id = ((hi as u128) << 64) | h.finish() as u128;
    let uuid = bevy::asset::uuid::Uuid::from_u128(id);
    let aid = AssetId::<bevy::shader::Shader>::Uuid { uuid };
    {
        let mut shaders = world.resource_mut::<Assets<bevy::shader::Shader>>();
        if shaders.get(aid).is_none() {
            // a WESL module's path is its import path (embedded://raea_sys_material/shader_material/m<hash> is raea_sys_material::shader_material::m<hash>)
            let mut shader = bevy::shader::Shader::from_wesl(module, format!("embedded://raea_sys_material/shader_material/m{id:032x}.wesl"));
            shader.shader_defs = defs;
            let _ = shaders.insert(aid, shader);
        }
    }
    let alpha = match o.get("alpha").and_then(Value::as_str).unwrap_or("opaque") {
        "opaque" => AlphaMode::Opaque,
        "blend" => AlphaMode::Blend,
        "add" => AlphaMode::Add,
        "premultiplied" => AlphaMode::Premultiplied,
        a => return Err(format!("a ShaderMaterial's alpha is opaque, blend, add or premultiplied, got {a:?}")),
    };
    let mut slots = [Vec4::ZERO; SLOTS];
    for (i, u) in us.iter().enumerate() {
        slots[i] = u.value;
    }
    let mut t = texs.into_iter().map(|x| Some(x.1));
    Ok(ShaderMaterial {
        slots,
        tex0: t.next().flatten(),
        tex1: t.next().flatten(),
        tex2: t.next().flatten(),
        tex3: t.next().flatten(),
        arr: arr.map(|a| a.1),
        shader: id,
        alpha,
        double_sided: o.get("double_sided").and_then(Value::as_bool).unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_module_has_accessors_bindings_and_an_entry() {
        let m = super::module("fn shade(in: VertexOutput) -> vec4<f32> { return u_tint(); }", &[("tint".into(), "vec4<f32>"), ("speed".into(), "f32")], &["noise".into()], Some("frames"));
        assert!(m.contains("fn u_tint() -> vec4<f32> { return raea_slots.s[0]; }"));
        assert!(m.contains("fn u_speed() -> f32 { return raea_slots.s[1].x; }"));
        assert!(m.contains("@binding(1) var tex_noise: texture_2d<f32>;"));
        assert!(m.contains("@fragment\nfn fragment(in: VertexOutput)"));
        assert!(m.contains("@binding(9) var tex_frames: texture_2d_array<f32>;"));
    }

    #[test]
    fn the_module_imports_and_binds_in_wesl() {
        let m = super::module("fn shade(in: VertexOutput) -> vec4<f32> { return u_tint(); }", &[("tint".into(), "vec4<f32>")], &["pic".into()], None);
        assert!(m.starts_with("import bevy_pbr::render::forward_io::VertexOutput;"));
        assert!(m.contains("@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> raea_slots"));
        assert!(m.contains("@group(constants::MATERIAL_BIND_GROUP) @binding(1) var tex_pic"));
        assert!(!m.contains("#import"));
    }
}
