// <Instances> in the three.js host: its placements ride in data-at, apart from its props (sdk/render instances.tsx), and
// the host draws a copy at each one as an InstancedMesh, not one mesh at the origin.
import { describe, expect, test } from "bun:test";
import * as THREE from "three";
import { Instances, Sphere } from "raea/3d";
import { StandardMaterial } from "raea/pbr";
import { Scene3d } from "../src/three/scene";
import { page } from "../src/export/dom";

function meshesOf(node: React.ReactNode) {
  const host = new Scene3d();
  host.sync(page(node) as unknown as Element);
  const out: THREE.Mesh[] = [];
  host.scene.traverse((o) => void ((o as THREE.Mesh).isMesh && out.push(o as THREE.Mesh)));
  return { host, out };
}

const at = [{ position: [1, 2, 0] as [number, number, number] }, { position: [-1, 0, 0.5] as [number, number, number] }, { position: [0, -2, 0] as [number, number, number] }];

describe("Instances", () => {
  test("a copy at each placement, in one InstancedMesh", () => {
    const { out } = meshesOf(
      <Instances at={at}>
        <Sphere radius={0.1} />
        <StandardMaterial base_color="#ff0000" unlit />
      </Instances>,
    );
    const inst = out.filter((m) => (m as THREE.InstancedMesh).isInstancedMesh) as THREE.InstancedMesh[];
    expect(inst.length).toBe(1);
    expect(inst[0].count).toBe(3);
    const m = new THREE.Matrix4();
    const p = new THREE.Vector3();
    inst[0].getMatrixAt(1, m);
    p.setFromMatrixPosition(m);
    expect(p.toArray()).toEqual([-1, 0, 0.5]);
  });

  test("moved placements move the copies", () => {
    const { host } = meshesOf(
      <Instances at={at}>
        <Sphere radius={0.1} />
        <StandardMaterial base_color="#ff0000" unlit />
      </Instances>,
    );
    host.sync(page(
      <Instances at={at.map((a) => ({ position: [a.position[0] + 1, a.position[1], a.position[2]] as [number, number, number] }))}>
        <Sphere radius={0.1} />
        <StandardMaterial base_color="#ff0000" unlit />
      </Instances>,
    ) as unknown as Element);
    const inst: THREE.InstancedMesh[] = [];
    host.scene.traverse((o) => void ((o as THREE.InstancedMesh).isInstancedMesh && inst.push(o as THREE.InstancedMesh)));
    const m = new THREE.Matrix4();
    inst[0].getMatrixAt(0, m);
    expect(new THREE.Vector3().setFromMatrixPosition(m).toArray()).toEqual([2, 2, 0]);
  });
});
