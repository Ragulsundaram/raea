// RR1: a grade ramp over 60 frames (saturation 0 to 2, temperature -0.3 to 0.3, bloom 0.1 to 1, fog colour from blue
// to orange), each frame's own. Rendered as one 60-frame piece and as 60 stills alone, the two must match.
import { AbsoluteFill, interpolate, registerRoot, Composition, useCurrentFrame } from "raea";
import { Bloom, Canvas3D, Fog, Grade, ToneMapping, manifold } from "raea/3d";
import { Camera3d } from "raea/camera";
import { AmbientLight, DirectionalLight } from "raea/light";
import { Mesh3d } from "raea/mesh";
import { StandardMaterial } from "raea/pbr";
import { Transform } from "raea/transform";

const GROUND = manifold.box(60, 1, 60).move(0, -0.5, 0);
const BOX = manifold.box(1.5, 1.5, 1.5);

function Ramp() {
  const f = useCurrentFrame();
  const t = (a: number, b: number) => interpolate(f, [0, 59], [a, b]);
  const hx = (v: number) => Math.round(v).toString(16).padStart(2, "0");
  const fog = `#${hx(t(60, 230))}${hx(t(110, 140))}${hx(t(220, 60))}`;
  return (
    <AbsoluteFill style={{ background: "#202830" }}>
      <Canvas3D>
        <Camera3d transform={Transform.fromXyz(6, 3, 8).lookingAt([0, 0.8, 0], [0, 1, 0])} projection={{ Perspective: { fov: 0.8, near: 0.1, far: 100 } }} />
        <DirectionalLight name="sun" transform={Transform.fromXyz(10, 20, 5).lookingAt([0, 0, 0], [0, 1, 0])} illuminance={20000} shadow_maps_enabled />
        <AmbientLight brightness={200} />
        <Mesh3d name="ground" shape={GROUND}><StandardMaterial base_color="#6a7a50" perceptual_roughness={0.9} /></Mesh3d>
        <Mesh3d name="red" shape={BOX} transform={Transform.fromXyz(-1.5, 0.75, 0)}><StandardMaterial base_color="#d03020" /></Mesh3d>
        <Mesh3d name="lamp" shape={BOX} transform={Transform.fromXyz(1.5, 0.75, -1)}><StandardMaterial base_color="#ffffff" emissive="#ffe0a0" emissive_intensity={8} /></Mesh3d>
        <ToneMapping mode="agx" exposure={0.5} />
        <Bloom intensity={t(0.1, 1)} threshold={0.8} />
        <Fog color={fog} near={4} far={30} />
        <Grade temperature={t(-0.3, 0.3)} saturation={t(0, 2)} midtones={{ contrast: t(0.8, 1.2) }} vignette={t(0, 0.6)} />
      </Canvas3D>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="ramp" component={Ramp} width={640} height={360} fps={30} durationInFrames={60} />);
