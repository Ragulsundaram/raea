// The coverage, measured (docs/r3f.feature R3F7): every kind raea's SDK renders has a row in the three host's table and
// is drawn by it (or named as an effect or engine-only); every three.js class an R3F scene can hold has a row in the
// input's table and is read by the input.
import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import * as THREE from "three";
import { KINDS } from "../src/three/kinds";
import { DRAWN_EFFECTS } from "../src/three/effects";
import { CLASSES } from "../src/input/kinds";
import { Scene3d } from "../src/three/scene";
import { parse } from "../src/export/dom";
import { standInImages } from "../src/input/texture";

// no DOM here: pictures are stand-ins carrying their url, as in a native render
standInImages();

const SDK = resolve(import.meta.dir, "../..");

/** every kind the SDK's components can render: node3d("Kind"), lensNode("Kind"), data-3d="Kind", geometry("Kind") */
function sdkKinds(): Set<string> {
  const out = new Set<string>();
  const walk = (d: string) => {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (f === "node_modules" || f === "test" || f === "tests" || f === "conformance" || f === "bench") continue;
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.(tsx?|ts)$/.test(f)) {
        const src = readFileSync(p, "utf8");
        for (const m of src.matchAll(/(?:node3d|lensNode)\("([A-Z][A-Za-z0-9]+)"|data-3d="([A-Z][A-Za-z0-9]+)"|geometry<[A-Za-z0-9 &{}?:[\],]+>\("([A-Z][A-Za-z0-9]+)"/g)) out.add(m[1] ?? m[2] ?? m[3]);
      }
    }
  };
  for (const pkg of ["render", "raea", "physics", "character", "room", "video", "app"]) walk(join(SDK, pkg, "src"));
  // the kinds named through a variable: the toon material (node3d(owner)) and motion blur (node3d(owner))
  for (const k of ["MeshToonMaterial", "MotionBlur"]) out.add(k);
  return out;
}

describe("raea → three.js coverage", () => {
  const kinds = sdkKinds();

  test("the SDK's kinds were found (the scan works)", () => {
    for (const k of ["Mesh", "MeshStandardMaterial", "DirectionalLight", "Model", "Bloom", "Splat", "Grass", "Fisheye"]) expect(kinds.has(k), k).toBe(true);
    expect(kinds.size).toBeGreaterThan(50);
  });

  test("every kind raea renders has a row in three/kinds.ts", () => {
    const missing = [...kinds].filter((k) => !(k in KINDS));
    expect(missing).toEqual([]);
  });

  test("every kind the host draws is drawn without a warning", () => {
    const warned: string[] = [];
    const warn = console.warn;
    console.warn = (m: string) => warned.push(String(m));
    try {
      for (const [kind, row] of Object.entries(KINDS)) {
        if (row.status === "effect" || row.status === "engine") continue;
        const host = new Scene3d();
        const props = SAMPLE[kind] ?? {};
        const inner = INSIDE[kind];
        const html = inner ? inner(kind, props) : `<div data-3d="${kind}" data-props="${esc(JSON.stringify(props))}"></div>`;
        host.sync(parse(html) as unknown as Element);
        const drawnBy = warned.filter((w) => w.includes(kind) && /isn't a kind|runs in raea's engine/.test(w));
        expect(drawnBy, kind).toEqual([]);
      }
    } finally {
      console.warn = warn;
    }
  });

  test("effects are listed for the app's composer, engine kinds named", () => {
    const host = new Scene3d();
    host.sync(parse(`<div data-3d="Bloom" data-props="${esc(JSON.stringify({ intensity: 0.3 }))}"></div><div data-3d="Fisheye" data-props="{}"></div>`) as unknown as Element);
    expect(host.effects.map((e) => e.kind)).toEqual(["Bloom", "Fisheye"]);
    expect(host.effects[0].props).toEqual({ intensity: 0.3 });
  });

  test("a Model's `at` draws a copy at each placement", () => {
    const host = new Scene3d();
    const at = [0, 0, 0, 0, 0, 0, 1, 1, 1, 2, 0, 0, 0, 0, 0, 1, 1, 1, 4, 0, 0, 0, 0, 0, 1, 1, 1];
    host.sync(parse(div("Model", { src: "x.glb", at })) as unknown as Element);
    const roots: number[] = [];
    host.scene.traverse((o) => o.type === "Group" && o.parent === host.scene && roots.push(o.position.x));
    expect(roots.sort()).toEqual([0, 2, 4]);
  });

  test("every effect row is one the composer draws", () => {
    const effects = Object.entries(KINDS).filter(([, r]) => r.status === "effect").map(([k]) => k).sort();
    expect(effects).toEqual([...DRAWN_EFFECTS].sort());
  });
});

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
const div = (kind: string, props: object, inner = "", shape?: string) => `<div data-3d="${kind}" data-props="${esc(JSON.stringify(props))}"${shape ? ` data-shape="${esc(shape)}"` : ""}>${inner}</div>`;

/** props a kind needs to be drawn at all */
const SAMPLE: Record<string, object> = {
  ImagePlane: { src: "x.png" }, Decal: { src: "x.png" }, Model: { src: "x.glb" }, Environment: { src: "sky.hdr" },
  Plate: { src: "x.mp4" }, Splat: { src: "x.ply" }, Sprite: { src: "x.png" }, Refinish: { materials: [{ color: "#ff0000" }] },
  Grass: { spec: { area: [0, 0, 1, 1], grid: [2, 2], height: [0, 0, 0, 0], density: [10, 10, 10, 10] } },
};
/** kinds that live inside another part */
const INSIDE: Record<string, (kind: string, props: object) => string> = {
  ...Object.fromEntries(["Box", "Sphere", "Plane", "Cylinder", "Cone", "Torus", "Capsule", "BufferMesh", "Text3D", "MeshStandardMaterial", "MeshPhysicalMaterial", "MeshBasicMaterial", "MeshToonMaterial"].map((k) => [k, (kind: string, props: object) => div("Mesh", {}, div(kind, props))])),
};

describe("R3F → raea coverage", () => {
  const classes = Object.entries(THREE).filter(([, v]) => typeof v === "function" && v.prototype && (v.prototype instanceof THREE.Object3D || v === THREE.Object3D || v.prototype instanceof THREE.Material || v === THREE.Material));

  test("three's scene classes were found (the scan works)", () => {
    expect(classes.map(([k]) => k)).toEqual(expect.arrayContaining(["Mesh", "Points", "SpotLight", "MeshPhysicalMaterial", "BatchedMesh"]));
  });

  test("every Object3D and Material class in three has a row in input/kinds.ts", () => {
    const missing = classes.map(([k]) => k).filter((k) => !(k in CLASSES));
    expect(missing).toEqual([]);
  });
});
