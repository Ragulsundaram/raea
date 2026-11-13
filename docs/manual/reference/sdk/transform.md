---
title: "raea/transform"
summary: "Where things are: Bevy's Transform and Quat, and nesting."
order: 67
sources: ["sdk/raea/src/transform.ts"]
---

`Transform` and `Quat` are Bevy's, with Bevy's methods: `Transform.fromXyz(...)`, `.lookingAt(...)`,
`.withRotation(...)`, `Quat.fromEuler(...)` and the rest. Every 3D component takes a `transform`, and children of an
entity are placed relative to it.

```ts
import { Quat, Transform } from "raea/transform";

const camera = Transform.fromXyz(0, 1.6, 5).lookingAt([0, 0.5, 0]);
const turned = Transform.IDENTITY.withRotation(Quat.fromEuler("XYZ", 0, Math.PI / 4, 0));
```

<!-- generated:begin by tools/manual-ref/sdk.ts from the SDK's exports and doc comments; edit outside this block -->

```ts
import { eulerOf, Quat, surfaceTurn, Transform } from "raea/transform"
```

`raea/transform` exports 2 functions, 2 values, 4 types. Bevy's counterpart: `bevy_transform`. Source: [sdk/raea/src/transform.ts](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/src/transform.ts).

## Functions

### `eulerOf`

A part's Euler angles of a quaternion: a composition's `rotation` [x, y, z] is composed as three.js composes it,
Quat::from_euler(EulerRot::XYZ, x, y, z) (runtime/scene `local`; the YXZ rot_x/rot_y/rot_z of
raea-render's rows are read back from the world matrix), so this is that order's inverse.

```ts
function eulerOf(q: [number, number, number, number]): Vec3
```

Used in: [3d/3d_scene](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/3d_scene.tsx), [shader_advanced/compute_mesh](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/shader_advanced/compute_mesh.tsx), [shader_advanced/custom_render_phase](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/shader_advanced/custom_render_phase.tsx).

### `surfaceTurn`

The turn that takes +y to `normal`, with the picture's top (a decal's -z) toward world up on a wall, toward -z on a
floor, turned `spin` about the normal: a quaternion [x, y, z, w].

```ts
function surfaceTurn(normal: Vec3, spin?: number): [number, number, number, number]
```

## Values

### `Quat`

A rotation as glam's Quat: [x, y, z, w].
glam's Quat constructors, by their Rust names in camelCase.

Used in: [ui/render_ui_to_texture](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/ui/render_ui_to_texture.tsx), [3d/3d_shapes](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/3d_shapes.tsx), [3d/texture](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/texture.tsx), [3d/shadow_caster_receiver](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/shadow_caster_receiver.tsx), [3d/split_screen](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/split_screen.tsx), [3d/spotlight](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/spotlight.tsx), [3d/mesh_ray_cast](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/mesh_ray_cast.tsx), [3d/3d_scene](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/3d_scene.tsx), and 24 more.

### `Transform`

Bevy's Transform: the component's handle for a program (`with(Transform, Transform.fromXyz(1, 0, 0))`) and its
builders for a scene (`<Mesh3d transform={Transform.fromXyz(0, 1, 0)}>`).

Used in: [ui/render_ui_to_texture](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/ui/render_ui_to_texture.tsx), [ui/widgets/viewport_node](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/ui/widgets/viewport_node.tsx), [3d/fog_volumes](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/fog_volumes.tsx), [3d/fog](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/fog.tsx), [3d/vertex_colors](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/vertex_colors.tsx), [3d/rotate_environment_map](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/rotate_environment_map.tsx), [3d/transparency_3d](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/transparency_3d.tsx), [3d/spherical_area_lights](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/bench/bevy-parity/3d/spherical_area_lights.tsx), and 72 more.

## Types

### `EulerRot`

glam's EulerRot, the intrinsic orders: "YXZ" turns about Y, then the turned X, then the turned Z.

```ts
type EulerRot = "XYZ" | "XZY" | "YXZ" | "YZX" | "ZXY" | "ZYX"
```

### `TransformBuilt`

A Transform built with Bevy's builders: each returns a new one.

| Field | Type | What it is |
| --- | --- | --- |
| `withTranslation` (required) | `(v: Vec3) => TransformBuilt` | the same transform, moved to `v` |
| `withRotation` (required) | `(q: Quat) => TransformBuilt` | the same transform, turned to `q` |
| `withScale` (required) | `(s: number \| Vec3) => TransformBuilt` | the same transform, scaled to `s` (one number for all three axes) |
| `lookingAt` (required) | `(target: Vec3, up?: Vec3 \| undefined) => TransformBuilt` | the same transform, turned so its -Z faces `target` with `up` (+Y) up, as Bevy's looking_at |
| `translation` (required) | `Vec3` | where it is, metres (x, y, z) |
| `rotation` (required) | `Quat` | its rotation as a quaternion [x, y, z, w] |
| `scale` (required) | `Vec3` | its scale along x, y and z |

### `TransformValue`

A Transform's value, as Bevy reflects it.

| Field | Type | What it is |
| --- | --- | --- |
| `translation` | `Vec3` | where it is, metres (x, y, z) |
| `rotation` | `Quat` | its rotation as a quaternion [x, y, z, w] |
| `scale` | `Vec3` | its scale along x, y and z |

### `Vec3`

```ts
type Vec3 = [number, number, number]
```

<!-- generated:end -->
