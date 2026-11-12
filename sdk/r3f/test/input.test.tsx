// R3F as raea's input (docs/r3f.feature R3F1-R3F3): the clock, the scene graph's harder parts, textures and units.
import { describe, expect, test } from "bun:test";
import { useFrame } from "@react-three/fiber";
import { createContext, useContext, useRef } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import * as THREE from "three";
import { Clock, useCurrentFrame } from "@raea/app/clock";
import { convert } from "../src/input/scene";
import { end, stepTo } from "../src/input/session";
import { pngOf, srcOf } from "../src/input/texture";
import { roughnessOfShininess } from "../src/input/material";
import { FiberOf, useBridge } from "../src/input/bridge";
import { EXPOSURE, relativeOf, toRaea, toThree } from "../src/units";
import type { Part } from "../src/parts";
import type { ReactNode } from "react";

const flat = (parts: Part[]): Part[] => parts.flatMap((p) => [p, ...flat(p.children ?? [])]);
let n = 0;
function partsOf(scene: (i: number) => ReactNode, frame = 0, config = {}) {
  const key = `t${n++}`;
  const s = stepTo(key, frame, 24, 640, 360, config, scene);
  const st = s.state as unknown as { scene: THREE.Scene; camera: THREE.Camera; gl: THREE.WebGLRenderer };
  const out = convert(st.scene, st.camera, st.gl, 360);
  return { ...out, key, st, s };
}

describe("units", () => {
  test("raea's units and three's are each other's inverse, and Bevy's exposure", () => {
    expect(EXPOSURE).toBeCloseTo(1 / (1.2 * 2 ** 9.7), 12);
    for (const k of ["DirectionalLight", "PointLight", "SpotLight", "AmbientLight", "RectAreaLight"] as const) expect(toRaea(k, toThree(k, 3.7, 2), 2)).toBeCloseTo(3.7, 9);
    // a sun of raea's 1 (8000 lux at EV 9.7) is three's 8000 / (1.2 · 2^9.7)
    expect(toThree("DirectionalLight", 1)).toBeCloseTo(8000 * EXPOSURE, 9);
    // raea/light's Bevy-shaped lights carry their own units: lux, lumens, nits
    expect(relativeOf("DirectionalLight", { lux: 16000, intensity: 1 })).toBe(2);
    expect(relativeOf("PointLight", { lumens: 450000 })).toBe(0.5);
    expect(relativeOf("AmbientLight", { nits: 800 })).toBe(2);
    expect(relativeOf("SpotLight", { intensity: 3 })).toBe(3);
  });
  test("Phong's shininess as a roughness: glossy is smooth, matte is rough", () => {
    expect(roughnessOfShininess(30)).toBeCloseTo(Math.sqrt(Math.sqrt(2 / 32)), 9);
    expect(roughnessOfShininess(1000)).toBeLessThan(0.25);
    expect(roughnessOfShininess(0)).toBe(1);
  });
});

function Spin() {
  const ref = useRef<THREE.Mesh>(null!);
  useFrame((_s, dt) => { ref.current.rotation.y += dt; });
  return <mesh ref={ref} name="spin"><boxGeometry /><meshBasicMaterial /></mesh>;
}

describe("the clock", () => {
  test("useFrame steps 1/fps a frame, and a seek back gives the same frame", () => {
    const key = "clock";
    const ry = (f: number) => {
      const s = stepTo(key, f, 24, 640, 360, {}, () => <Spin />);
      return (s.state.scene.getObjectByName("spin") as THREE.Mesh).rotation.y;
    };
    expect(ry(0)).toBeCloseTo(0, 9);
    expect(ry(24)).toBeCloseTo(1, 9);
    expect(ry(48)).toBeCloseTo(2, 9);
    // backwards: started again from 0 and stepped, the same as before
    expect(ry(24)).toBeCloseTo(1, 9);
    end(key);
  });

  test("raea's own hooks work inside the R3F scene (the context bridge), at each stepped frame", () => {
    const Theme = createContext("none");
    let seen: string[] = [];
    function Reader() {
      const f = useCurrentFrame();
      seen.push(`${useContext(Theme)}@${f}`);
      return null;
    }
    function Outer() {
      return (
        <FiberOf>
          {(fiber) => {
            const Inner = () => {
              const bridge = useBridge(fiber);
              seen = [];
              stepTo("bridge", 2, 24, 64, 64, {}, (i) => bridge(<Clock frame={i} fps={24} width={64} height={64} durationInFrames={10}><Reader /></Clock>));
              return null;
            };
            return <Inner />;
          }}
        </FiberOf>
      );
    }
    renderToStaticMarkup(<Theme value="dark"><Outer /></Theme>);
    expect(seen).toEqual(["dark@0", "dark@1", "dark@2"]);
    end("bridge");
  });
});

describe("the scene graph", () => {
  test("an InstancedMesh holding still is Instances; moving, a mesh a copy", () => {
    const geo = new THREE.BoxGeometry(), mat = new THREE.MeshStandardMaterial({ color: "#336699" });
    const im = new THREE.InstancedMesh(geo, mat, 3);
    const m = new THREE.Matrix4();
    for (let i = 0; i < 3; i++) im.setMatrixAt(i, m.makeTranslation(i * 2, 0, 0));
    const a = partsOf(() => <primitive object={im} />);
    const inst = flat(a.parts).find((p) => p.type === "Instances")!;
    expect((inst.props.at as unknown[]).length).toBe(3);
    expect((inst.props.at as { position: number[] }[])[2].position).toEqual([4, 0, 0]);
    im.setMatrixAt(1, m.makeTranslation(0, 5, 0));
    im.instanceMatrix.needsUpdate = true;
    const b = convert(a.st.scene, a.st.camera, a.st.gl, 360);
    const meshes = flat(b.parts).filter((p) => p.type === "Mesh");
    expect(meshes.length).toBe(3);
    expect(meshes[1].props.position).toEqual([0, 5, 0]);
    end(a.key);
  });

  test("a mesh swapped into the same place with another shader program is another part; the same program keeps its", () => {
    const vs = "void main() { gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }";
    const shaded = (fs: string, uniforms: Record<string, THREE.IUniform>) => new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.ShaderMaterial({ uniforms, vertexShader: vs, fragmentShader: fs }));
    const a = shaded("uniform float k; void main() { gl_FragColor = vec4(k); }", { k: { value: 0.5 } });
    const a2 = shaded("uniform float k; void main() { gl_FragColor = vec4(k); }", { k: { value: 0.9 } });
    const b = shaded("uniform mat4 m; void main() { gl_FragColor = m[0]; }", { m: { value: new THREE.Matrix4() } });
    const nameOf = (o: THREE.Mesh) => {
      const r = partsOf(() => <primitive object={o} />);
      end(r.key);
      return flat(r.parts).find((p) => p.type === "Mesh")!.props.name;
    };
    expect(nameOf(a)).toBe(nameOf(a2));
    expect(nameOf(a)).not.toBe(nameOf(b));
  });

  test("Points are quads facing the camera with three's size, lines are gizmo lines, a sprite a quad", () => {
    const pts = new THREE.BufferGeometry().setAttribute("position", new THREE.Float32BufferAttribute([0, 0, 0, 1, 0, 0], 3));
    const line = new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0, 0, 0), new THREE.Vector3(0, 1, 0), new THREE.Vector3(1, 1, 0)]);
    const r = partsOf(() => (
      <>
        <points geometry={pts}><pointsMaterial size={0.2} color="#ff0000" /></points>
        <lineSegments geometry={line}><lineBasicMaterial color="#00ff00" /></lineSegments>
        <primitive object={new THREE.Line(line, new THREE.LineBasicMaterial({ color: "#0000ff" }))} position={[0, 0, 1]} />
        <sprite position={[0, 2, 0]} scale={[2, 1, 1]}><spriteMaterial color="#ffffff" /></sprite>
      </>
    ), 0, { camera: { position: [0, 0, 5], fov: 50 } });
    const all = flat(r.parts);
    const quads = all.find((p) => p.type === "Mesh" && (p.children![0].props.positions as number[]).length === 24)!;
    expect(quads).toBeDefined();
    const pos = quads.children![0].props.positions as number[];
    // with attenuation, a point is size · tan(fov/2) across
    expect(pos[3] - pos[0]).toBeCloseTo(0.2 * Math.tan((50 * Math.PI) / 360), 6);
    const gizmos = all.filter((p) => p.type === "Gizmo");
    expect(gizmos[0].props.lines).toEqual([[[0, 0, 0], [0, 1, 0]]]);
    expect(gizmos[1].props.strip).toEqual([[0, 0, 1], [0, 1, 1], [1, 1, 1]]);
    expect(gizmos[1].props.color).toBe("#0000ff");
    const sprite = all.find((p) => p.type === "Mesh" && (p.children![0].props.positions as number[]).length === 12)!;
    expect(sprite.props.position).toEqual([0, 2, 0]);
    expect(sprite.props.scale).toEqual([2, 1, 1]);
    end(r.key);
  });

  test("a skinned mesh is posed on the CPU: its vertices follow its bones", () => {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute([0, 0, 0, 1, 0, 0, 0, 1, 0], 3));
    geo.setAttribute("skinIndex", new THREE.Uint16BufferAttribute([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 4));
    geo.setAttribute("skinWeight", new THREE.Float32BufferAttribute([1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0], 4));
    geo.computeVertexNormals();
    const bone = new THREE.Bone();
    const mesh = new THREE.SkinnedMesh(geo, new THREE.MeshStandardMaterial());
    mesh.add(bone);
    mesh.bind(new THREE.Skeleton([bone]));
    bone.position.set(0, 3, 0);
    const r = partsOf(() => <primitive object={mesh} />);
    const b = flat(r.parts).find((p) => p.type === "BufferMesh")!;
    expect((b.props.positions as number[]).slice(0, 3).map((x) => Math.round(x * 1e6) / 1e6)).toEqual([0, 3, 0]);
    end(r.key);
  });

  test("a multi-material mesh is a mesh per geometry group", () => {
    const geo = new THREE.BoxGeometry();
    const r = partsOf(() => <mesh geometry={geo} material={[0, 1, 2, 3, 4, 5].map((i) => new THREE.MeshBasicMaterial({ color: i % 2 ? "#ff0000" : "#0000ff" }))} />);
    const meshes = flat(r.parts).filter((p) => p.type === "Mesh");
    expect(meshes.length).toBe(6);
    expect(meshes.map((m) => (m.children![0].props.indices as number[]).length)).toEqual([6, 6, 6, 6, 6, 6]);
    end(r.key);
  });
  test("an opacity past 1 or below 0 (an overshooting easing) is clamped, as three draws it", () => {
    const r = partsOf(() => <>
      <mesh><boxGeometry /><meshBasicMaterial transparent opacity={1.03} /></mesh>
      <mesh><boxGeometry /><meshStandardMaterial transparent opacity={-0.02} /></mesh>
    </>);
    const ops = flat(r.parts).filter((p) => p.type === "Mesh").map((m) => m.children!.find((c) => "opacity" in c.props)!.props.opacity);
    expect(ops).toEqual([1, 0]);
    end(r.key);
  });
});

describe("textures", () => {
  test("a DataTexture is a PNG raea loads; a loaded picture keeps its url", () => {
    const t = new THREE.DataTexture(new Uint8Array([255, 0, 0, 255, 0, 255, 0, 255]), 2, 1);
    t.needsUpdate = true;
    const src = srcOf(t)!;
    expect(src.startsWith("data:image/png;base64,")).toBe(true);
    const bytes = Uint8Array.from(atob(src.split(",")[1]), (c) => c.charCodeAt(0));
    expect([...bytes.subarray(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
    const img = new THREE.Texture({ src: "media/wood.jpg", width: 4, height: 4 } as unknown as HTMLImageElement);
    expect(srcOf(img)).toBe("media/wood.jpg");
    expect(pngOf(new Uint8Array(4), 1, 1).length).toBeGreaterThan(40);
  });
});
