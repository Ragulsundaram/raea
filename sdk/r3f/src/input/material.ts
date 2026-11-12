// A three.js material as raea's: the component that draws it the same way and its props. Standard and physical are
// Bevy's StandardMaterial on both sides; basic is unlit; Lambert is the standard surface with no gloss; Phong's
// shininess becomes a roughness (Blinn-Phong's exponent as a GGX roughness); toon is raea's toon surface with a band per
// step of its gradient map; normal, matcap, depth and distance materials are drawn as the unlit colour each puts on
// screen most (their per-pixel looks need three's shaders). A ShaderMaterial's GLSL doesn't run in Bevy: it is drawn
// with its `color` (or diffuse) uniform, unlit, and named once, with raea's <SurfaceShader> (WESL) as its port.
import * as THREE from "three";
import { placementOf, srcOf } from "./texture";
import { classicOf, nodeMaterialOf, type NodeAt } from "./tsl";
import { builtinPart, isShader, shaderPart } from "./shader";

export type Props = Record<string, unknown>;
export interface Part {
  /** the raea/3d component that draws it */
  type: string;
  props: Props;
  children?: Part[];
  /** a MeshNormalMaterial: the mesh's normals become its vertex colours */
  normals?: boolean;
  /** a ShadowMaterial: the mesh shows only the shadows falling on it (raea's shadow="catch") */
  catchShadow?: boolean;
}

const hex = (c: THREE.Color) => "#" + c.getHexString();
/** three draws an opacity outside [0, 1] clamped (an easing that overshoots gives 1.03); raea's materials refuse it */
const unit = (v: number) => Math.min(1, Math.max(0, v));
const side = (s: THREE.Side) => (s === THREE.BackSide ? "back" : s === THREE.DoubleSide ? "double" : undefined);

/** the props every surface shares: colour, opacity and blending, side, fog */
function drawn(m: THREE.Material & { color?: THREE.Color; map?: THREE.Texture | null }, p: Props) {
  if (m.color) p.color = hex(m.color);
  if (m.opacity < 1 || m.transparent) {
    p.opacity = unit(m.opacity);
    p.transparent = true;
  }
  if (m.blending === THREE.AdditiveBlending) p.alphaMode = "add";
  else if (m.blending === THREE.MultiplyBlending) p.alphaMode = "multiply";
  else if (m.alphaTest > 0) {
    p.alphaMode = "mask";
    p.alphaCutoff = m.alphaTest;
  }
  const s = side(m.side);
  if (s) p.side = s;
  if ((m as { fog?: boolean }).fog === false) p.fog = false;
  const map = srcOf(m.map);
  if (map) {
    p.map = map;
    Object.assign(p, placementOf(m.map!));
  }
}

type Std = THREE.MeshStandardMaterial;
function standard(m: Std, p: Props) {
  drawn(m, p);
  p.roughness = m.roughness;
  p.metalness = m.metalness;
  if (m.emissive && (m.emissive.r || m.emissive.g || m.emissive.b)) {
    p.emissive = hex(m.emissive);
    p.emissiveIntensity = m.emissiveIntensity;
  }
  const maps: [keyof Std, string][] = [["normalMap", "normalMap"], ["roughnessMap", "roughnessMap"], ["metalnessMap", "metalnessMap"], ["aoMap", "aoMap"], ["emissiveMap", "emissiveMap"], ["displacementMap", "depthMap"]];
  for (const [from, to] of maps) {
    const s = srcOf(m[from] as THREE.Texture | null);
    if (s) p[to] = s;
  }
  // three keeps metalness and roughness in one glTF texture's blue and green when both maps are the same one
  if (m.roughnessMap && m.roughnessMap === m.metalnessMap) {
    p.metallicRoughnessMap = p.roughnessMap;
    delete p.roughnessMap;
    delete p.metalnessMap;
  }
}

function physical(m: THREE.MeshPhysicalMaterial, p: Props) {
  standard(m, p);
  if (m.clearcoat) {
    p.clearcoat = m.clearcoat;
    p.clearcoatRoughness = m.clearcoatRoughness;
  }
  // drei's MeshTransmissionMaterial keeps its transmission in a uniform (three's own is 0, so three makes no pass
  // of its own): raea's is Bevy's transmission, which refracts what's behind as drei's buffer does
  const drei = (m as unknown as { uniforms?: Record<string, { value: unknown }> }).uniforms;
  const own = typeof drei?._transmission?.value === "number" ? (drei._transmission.value as number) : 0;
  if (own && !m.transmission) {
    p.transmission = own;
    p.thickness = m.thickness;
    const extras = ["chromaticAberration", "distortion", "temporalDistortion", "anisotropicBlur"].filter((k) => Number(drei?.[k]?.value ?? 0) !== 0);
    if (extras.length) warnOnce(`MeshTransmissionMaterial ${extras.join(", ")}`, "isn't drawn: Bevy's transmission refracts without it");
  } else if (m.transmission) {
    p.transmission = m.transmission;
    p.thickness = m.thickness;
  }
  if (m.ior !== 1.5) p.ior = m.ior;
  // three's specularIntensity scales the 4% a dielectric reflects at ior 1.5: Bevy's reflectance is 0.5 at 4%
  if (m.specularIntensity !== 1) p.reflectance = Math.min(1, 0.5 * m.specularIntensity);
  if (m.specularColor && hex(m.specularColor) !== "#ffffff") p.specularTint = hex(m.specularColor);
  if (m.anisotropy) p.anisotropy = { strength: m.anisotropy, rotation: m.anisotropyRotation };
  if (m.attenuationDistance !== Infinity) p.attenuation = { distance: m.attenuationDistance, color: hex(m.attenuationColor) };
}

/** Blinn-Phong's specular exponent as a GGX roughness (α² = 2 / (n + 2)) */
export const roughnessOfShininess = (n: number) => Math.min(1, Math.sqrt(Math.sqrt(2 / (Math.max(0, n) + 2))));

/** the toon steps a gradient map holds: its texels, one band each */
function toonBands(m: THREE.MeshToonMaterial): number {
  const g = m.gradientMap?.image as { width?: number } | undefined;
  return Math.max(1, Math.min(8, g?.width ?? 2));
}

/** the materials named once a session (their look isn't Bevy's) */
const warned = new Set<string>();
function warnOnce(type: string, why: string) {
  if (warned.has(type)) return;
  warned.add(type);
  console.warn(`raea/r3f: ${type} ${why}`);
}

/** a material as raea's: its component and props */
export function materialOf(m: THREE.Material, vertexColors: boolean, at?: NodeAt): Part {
  const p: Props = {};
  const t = m.type;
  switch (t) {
    case "MeshPhysicalMaterial":
      physical(m as THREE.MeshPhysicalMaterial, p);
      return { type: "MeshPhysicalMaterial", props: p };
    case "MeshStandardMaterial":
      standard(m as Std, p);
      return { type: "MeshStandardMaterial", props: p };
    case "MeshLambertMaterial": {
      const l = m as THREE.MeshLambertMaterial;
      drawn(l, p);
      p.roughness = 1;
      p.metalness = 0;
      if (l.emissive.r || l.emissive.g || l.emissive.b) Object.assign(p, { emissive: hex(l.emissive), emissiveIntensity: l.emissiveIntensity });
      return { type: "MeshStandardMaterial", props: p };
    }
    case "MeshPhongMaterial": {
      const ph = m as THREE.MeshPhongMaterial;
      drawn(ph, p);
      p.roughness = roughnessOfShininess(ph.shininess);
      p.metalness = 0;
      // Blinn-Phong's specular colour is its F0 (three's BRDF_BlinnPhong): Bevy's F0 is 0.16 reflectance² times the tint
      const sp = ph.specular, top = Math.max(sp.r, sp.g, sp.b);
      if (top > 0) {
        p.reflectance = Math.min(1, Math.sqrt(top / 0.16));
        if (sp.r !== sp.g || sp.g !== sp.b) p.specularTint = hex(new THREE.Color(sp.r / top, sp.g / top, sp.b / top));
      } else p.reflectance = 0;
      if (ph.emissive.r || ph.emissive.g || ph.emissive.b) Object.assign(p, { emissive: hex(ph.emissive), emissiveIntensity: ph.emissiveIntensity });
      const n = srcOf(ph.normalMap);
      if (n) p.normalMap = n;
      // (reflectance is the physical surface's)
      return { type: "MeshPhysicalMaterial", props: p };
    }
    case "MeshToonMaterial": {
      const tm = m as THREE.MeshToonMaterial;
      drawn(tm, p);
      for (const k of ["alphaMode", "alphaCutoff", "side", "fog", "repeat", "uvTransform", "sampler"]) delete p[k];
      p.steps = toonBands(tm);
      p.rim = false;
      p.highlight = false;
      p.outline = false;
      if (tm.emissive.r || tm.emissive.g || tm.emissive.b) Object.assign(p, { emissive: hex(tm.emissive), emissiveIntensity: tm.emissiveIntensity });
      return { type: "MeshToonMaterial", props: p };
    }
    case "MeshBasicMaterial":
    case "SpriteMaterial":
    case "PointsMaterial":
    case "LineBasicMaterial":
    case "LineDashedMaterial":
      drawn(m as THREE.MeshBasicMaterial, p);
      return { type: "MeshBasicMaterial", props: p };
    // three's own programs for these (its ShaderLib's matcap and normal), run by raea's engine as three runs them
    case "MeshMatcapMaterial":
    case "MeshNormalMaterial":
      return builtinPart(m as THREE.MeshMatcapMaterial);
    case "MeshDepthMaterial":
    case "MeshDistanceMaterial":
      warnOnce(t, "is drawn unlit grey (it is three's shadow and depth pass, not a surface)");
      return { type: "MeshBasicMaterial", props: { color: "#808080" } };
    case "ShadowMaterial": {
      const s = m as THREE.ShadowMaterial;
      return { type: "MeshBasicMaterial", props: { color: hex(s.color), opacity: unit(s.opacity), transparent: true }, catchShadow: true } as Part;
    }
    default: {
      // a TSL node material (three/webgpu): one with no node of its own is its classic material; one with a graph is
      // compiled to WGSL by three and run by raea's engine (tsl.ts), or, if its graph can't be, drawn flat below
      if ((m as { isNodeMaterial?: boolean }).isNodeMaterial) {
        const classic = classicOf(m);
        // (its `type` is three's accessor: defined on a view of it, not assigned)
        if (classic) return materialOf(Object.defineProperties(Object.create(m) as THREE.Material, { type: { value: classic }, isNodeMaterial: { value: false } }), vertexColors, at);
        const shader = at && nodeMaterialOf(m, at);
        if (shader) return shader;
      }
      // a ShaderMaterial or RawShaderMaterial: its GLSL run by raea's engine as three would run it (shader.ts)
      if (isShader(m)) return shaderPart(m);
      // a NodeMaterial whose graph can't be compiled, a library's own
      const u = (m as THREE.ShaderMaterial).uniforms ?? {};
      const c = (u.color ?? u.diffuse ?? u.uColor)?.value;
      warnOnce(t, "has a shader raea can't run: drawn unlit in its colour uniform");
      drawn(m as THREE.MeshBasicMaterial, p);
      if (c instanceof THREE.Color) p.color = hex(c);
      return { type: "MeshBasicMaterial", props: p };
    }
  }
  void vertexColors;
}
