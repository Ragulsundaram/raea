// The parity cases (docs/r3f.feature R3F8): each drawn by raea's engine (native.tsx, `raea render`) and by three.js in
// Chrome (the harness page), and the two compared by SSIM (tools/parity.sh). `in-*` are R3F scenes, raea's input;
// `out-*` are raea compositions, drawn in React Three Fiber by <Raea>.
import { useFrame } from "@react-three/fiber";
import { useRef, type ReactNode } from "react";
import * as THREE from "three";
import { useCurrentFrame } from "raea";
import {
  Bloom, Cuboid, Grade, LensDistortion, Fog, Sphere, Text3D, ToneMapping, Torus,
} from "raea/3d";
import {
  AmbientLight, DirectionalLight, Group, Mesh, MeshBasicMaterial, MeshPhysicalMaterial, MeshStandardMaterial,
  PerspectiveCamera, PointLight, RectAreaLight, SpotLight,
} from "@raea/render/three";
import { AbsoluteFill } from "raea";

export const SIZE = { width: 640, height: 360, fps: 24 };

// ---- R3F scenes ----
function Spinner() {
  const ref = useRef<THREE.Mesh>(null!);
  useFrame((_s, dt) => {
    ref.current.rotation.y += dt;
    ref.current.rotation.x += dt * 0.5;
  });
  return (
    <mesh ref={ref} position={[0, 0.8, 0]} castShadow receiveShadow>
      <boxGeometry args={[1, 1, 1]} />
      <meshStandardMaterial color="#e05a2b" roughness={0.4} />
    </mesh>
  );
}

const ground = (
  <mesh rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
    <planeGeometry args={[12, 12]} />
    <meshStandardMaterial color="#cfd6e0" roughness={0.9} />
  </mesh>
);

const instanced = (() => {
  const m = new THREE.InstancedMesh(new THREE.SphereGeometry(0.18, 24, 12), new THREE.MeshStandardMaterial({ color: "#f2cc60", roughness: 0.3 }), 9);
  const o = new THREE.Object3D();
  for (let i = 0; i < 9; i++) {
    o.position.set(-2 + (i % 3) * 0.5, 0.2, 1 + Math.floor(i / 3) * 0.5);
    o.updateMatrix();
    m.setMatrixAt(i, o.matrix);
  }
  m.castShadow = true;
  return m;
})();

export interface InputCase {
  canvas: Record<string, unknown>;
  scene: () => ReactNode;
  frame: number;
}

export const INPUTS: Record<string, InputCase> = {
  basic: {
    canvas: { camera: { position: [3, 2.5, 4], fov: 45 }, shadows: true },
    frame: 24,
    scene: () => (
      <>
        <color attach="background" args={["#1d2330"]} />
        <ambientLight intensity={0.4} />
        <directionalLight position={[3, 5, 2]} intensity={2.5} castShadow />
        <pointLight position={[-2, 1.5, 1]} intensity={6} color="#58a6ff" />
        <Spinner />
        {ground}
        <mesh position={[-1.6, 0.5, -0.5]} castShadow>
          <sphereGeometry args={[0.5, 48, 24]} />
          <meshPhysicalMaterial color="#3fb950" clearcoat={1} roughness={0.2} />
        </mesh>
        <mesh position={[1.6, 0.6, -0.4]} castShadow>
          <torusKnotGeometry args={[0.35, 0.12, 128, 16]} />
          <meshStandardMaterial color="#d2a8ff" metalness={0.7} roughness={0.25} />
        </mesh>
      </>
    ),
  },
  lights: {
    canvas: { camera: { position: [0, 3, 6], fov: 50 }, shadows: true },
    frame: 0,
    scene: () => (
      <>
        <color attach="background" args={["#0b0e14"]} />
        <fog attach="fog" args={["#0b0e14", 6, 14]} />
        <spotLight position={[0, 4, 1]} angle={0.6} penumbra={0.4} intensity={40} castShadow />
        <hemisphereLight args={["#bcd3ff", "#3a2a1a", 0.6]} />
        <rectAreaLight position={[-2, 1, 2]} width={1.5} height={1} intensity={6} color="#ff6040" onUpdate={(l) => l.lookAt(0, 0.5, 0)} />
        {ground}
        <mesh position={[0, 0.6, 0]} castShadow>
          <cylinderGeometry args={[0.4, 0.6, 1.2, 48]} />
          <meshLambertMaterial color="#e6e6e6" />
        </mesh>
        <mesh position={[1.5, 0.5, 0.5]} castShadow>
          <icosahedronGeometry args={[0.5, 1]} />
          <meshPhongMaterial color="#4ea1ff" shininess={60} flatShading />
        </mesh>
      </>
    ),
  },
  things: {
    canvas: { camera: { position: [0, 2.2, 5], fov: 45 }, shadows: true },
    frame: 0,
    scene: () => (
      <>
        <color attach="background" args={["#202830"]} />
        <ambientLight intensity={0.5} />
        <directionalLight position={[2, 4, 3]} intensity={2} castShadow />
        {ground}
        <primitive object={instanced} />
        <mesh position={[0.6, 0.5, 0]} castShadow>
          <boxGeometry args={[0.8, 0.8, 0.8]} />
          {["#e05a2b", "#3fb950", "#58a6ff", "#d2a8ff", "#f2cc60", "#ffffff"].map((c, i) => <meshStandardMaterial key={i} attach={`material-${i}`} color={c} />)}
        </mesh>
        <lineSegments position={[2, 0.01, 0]}>
          <edgesGeometry args={[new THREE.BoxGeometry(1, 1, 1)]} />
          <lineBasicMaterial color="#ffffff" />
        </lineSegments>
        <sprite position={[-0.8, 1.6, 0]} scale={[0.6, 0.6, 1]}>
          <spriteMaterial color="#ff4fa3" />
        </sprite>
      </>
    ),
  },
  // a ShadowMaterial floor: only the box's shadow is drawn, over the page's colour
  catch: {
    canvas: { camera: { position: [0, 2.5, 4.5], fov: 45 }, shadows: true },
    frame: 0,
    scene: () => (
      <>
        <color attach="background" args={["#d8c9a8"]} />
        <ambientLight intensity={0.5} />
        <directionalLight position={[2, 5, 1]} intensity={2.5} castShadow />
        <mesh rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
          <planeGeometry args={[12, 12]} />
          <shadowMaterial color="#1a1030" opacity={0.6} />
        </mesh>
        <mesh position={[0, 0.6, 0]} rotation={[0, 0.5, 0]} castShadow>
          <boxGeometry args={[1, 1.2, 1]} />
          <meshStandardMaterial color="#e05a2b" roughness={0.5} />
        </mesh>
      </>
    ),
  },
};

// ---- raea compositions ----
function OutBasic({ tone = "aces" }: { tone?: "aces" | "tony" }) {
  const f = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: "#1d2330" }}>
      <PerspectiveCamera position={[3, 2.5, 4]} lookAt={[0, 0.5, 0]} fov={45} />
      {tone === "aces" && <ToneMapping mode="aces" />}
      <AmbientLight intensity={0.4} />
      <DirectionalLight position={[3, 5, 2]} target={[0, 0, 0]} intensity={2.5} />
      <PointLight position={[-2, 1.5, 1]} intensity={6} color="#58a6ff" />
      <Mesh name="ground" rotation={[-Math.PI / 2, 0, 0]}>
        <Cuboid args={[12, 12, 0.001]} />
        <MeshStandardMaterial color="#cfd6e0" roughness={0.9} />
      </Mesh>
      <Mesh name="box" position={[0, 0.8, 0]} rotation={[f / 48, f / 24, 0]}>
        <Cuboid args={[1, 1, 1]} />
        <MeshStandardMaterial color="#e05a2b" roughness={0.4} />
      </Mesh>
      <Mesh name="ball" position={[-1.6, 0.5, -0.5]}>
        <Sphere args={[0.5]} />
        <MeshPhysicalMaterial color="#3fb950" clearcoat={1} roughness={0.2} />
      </Mesh>
      <Group position={[1.6, 0.6, -0.4]} rotation={[0.4, 0, 0]}>
        <Mesh name="ring">
          <Torus args={[0.4, 0.15]} />
          <MeshStandardMaterial color="#d2a8ff" metalness={0.7} roughness={0.25} />
        </Mesh>
      </Group>
    </AbsoluteFill>
  );
}

function OutLights() {
  return (
    <AbsoluteFill style={{ background: "#0b0e14" }}>
      <PerspectiveCamera position={[0, 3, 6]} lookAt={[0, 0.5, 0]} fov={50} />
      <ToneMapping mode="aces" />
      <Fog color="#0b0e14" near={6} far={14} />
      <SpotLight position={[0, 4, 1]} target={[0, 0, 0]} angle={0.6} penumbra={0.4} intensity={1.5} castShadow />
      <RectAreaLight position={[-2, 1, 2]} target={[0, 0.5, 0]} width={1.5} height={1} intensity={1} color="#ff6040" />
      <AmbientLight intensity={0.2} />
      <Mesh name="ground" rotation={[-Math.PI / 2, 0, 0]}>
        <Cuboid args={[12, 12, 0.001]} />
        <MeshStandardMaterial color="#cfd6e0" roughness={0.9} />
      </Mesh>
      <Mesh name="glow" position={[1.5, 0.5, 0.5]}>
        <Sphere args={[0.4]} />
        <MeshBasicMaterial color="#4ea1ff" />
      </Mesh>
      <Mesh name="sign" position={[-0.9, 0.3, 0]}>
        <Text3D size={0.5} depth={0.15}>R3F</Text3D>
        <MeshStandardMaterial color="#f2cc60" roughness={0.5} />
      </Mesh>
    </AbsoluteFill>
  );
}

function OutEffects({ fx }: { fx: "bloom" | "grade" | "lens" }) {
  return (
    <AbsoluteFill style={{ background: "#0b0e14" }}>
      <PerspectiveCamera position={[0, 1.5, 5]} lookAt={[0, 0.5, 0]} fov={45} />
      <ToneMapping mode="aces" />
      {fx === "bloom" && <Bloom intensity={2} threshold={0.6} />}
      {fx === "grade" && <Grade saturation={1.3} vignette={0.5} midtones={{ contrast: 1.1 }} />}
      {fx === "lens" && <LensDistortion k1={0.15} />}
      <AmbientLight intensity={0.3} />
      <DirectionalLight position={[2, 4, 3]} target={[0, 0, 0]} intensity={1.5} />
      <Mesh name="ground" rotation={[-Math.PI / 2, 0, 0]}>
        <Cuboid args={[12, 12, 0.001]} />
        <MeshStandardMaterial color="#8090a0" roughness={0.9} />
      </Mesh>
      <Mesh name="lamp" position={[-1, 0.6, 0]}>
        <Sphere args={[0.5]} />
        <MeshStandardMaterial color="#000000" emissive="#ffb050" emissiveIntensity={4} />
      </Mesh>
      <Mesh name="box" position={[1, 0.5, 0]} rotation={[0, 0.6, 0]}>
        <Cuboid args={[1, 1, 1]} />
        <MeshStandardMaterial color="#58a6ff" roughness={0.4} />
      </Mesh>
    </AbsoluteFill>
  );
}

function OutCatch() {
  return (
    <AbsoluteFill style={{ background: "#d8c9a8" }}>
      <PerspectiveCamera position={[0, 2.5, 4.5]} lookAt={[0, 0.3, 0]} fov={45} />
      <ToneMapping mode="aces" />
      <AmbientLight intensity={0.5} />
      <DirectionalLight position={[2, 5, 1]} target={[0, 0, 0]} intensity={2.5} castShadow />
      <Mesh name="floor" rotation={[-Math.PI / 2, 0, 0]} shadow="catch">
        <Cuboid args={[12, 12, 0.001]} />
        <MeshBasicMaterial color="#1a1030" opacity={0.6} />
      </Mesh>
      <Mesh name="box" position={[0, 0.6, 0]} rotation={[0, 0.5, 0]}>
        <Cuboid args={[1, 1.2, 1]} />
        <MeshStandardMaterial color="#e05a2b" roughness={0.5} />
      </Mesh>
    </AbsoluteFill>
  );
}

export const OUTPUTS: Record<string, { component: () => ReactNode; frame: number }> = {
  basic: { component: () => <OutBasic />, frame: 24 },
  // raea's default tone mapping, TonyMcMapface
  tony: { component: () => <OutBasic tone="tony" />, frame: 24 },
  lights: { component: OutLights, frame: 0 },
  // one effect a case: each pass measured on its own
  bloom: { component: () => <OutEffects fx="bloom" />, frame: 0 },
  grade: { component: () => <OutEffects fx="grade" />, frame: 0 },
  lens: { component: () => <OutEffects fx="lens" />, frame: 0 },
  // shadow="catch" (three's ShadowMaterial): only the shadow on the floor
  catch: { component: OutCatch, frame: 0 },
};
