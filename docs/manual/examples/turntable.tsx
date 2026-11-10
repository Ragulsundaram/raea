// check: frames 0,45,89
import { AbsoluteFill, Composition, Easing, interpolate, registerRoot, useCurrentFrame, useVideoConfig } from "raea";
import { Cylinder, Entity } from "raea/3d";
import { StandardMaterial } from "raea/pbr";
import { Cuboid, Mesh3d } from "raea/mesh";
import { AmbientLight, DirectionalLight } from "raea/light";
import { Quat, Transform } from "raea/transform";
import { Camera3d } from "raea/camera";

function Turntable() {
  const f = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const turn = (f / fps) * (Math.PI / 3); // a sixth of a turn a second
  const z = interpolate(f, [0, durationInFrames - 1], [7, 5], { easing: Easing.out(Easing.cubic), extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ background: "#14161c" }}>
      <Camera3d transform={Transform.fromXyz(0, 1.6, z).lookingAt([0, 0.5, 0])} projection={{ Perspective: { fov: (40 * Math.PI) / 180 } }} />
      <AmbientLight brightness={160} />
      <DirectionalLight transform={Transform.fromXyz(3, 5, 4).lookingAt([0, 0, 0])} illuminance={11200} shadow_maps_enabled />
      <Entity transform={Transform.IDENTITY.withRotation(Quat.fromEuler("XYZ", 0, turn, 0))}>
        <Mesh3d transform={Transform.fromXyz(0, -0.1, 0)}>
          <Cylinder radius={1.4} half_height={0.1} />
          <StandardMaterial base_color="#2b2f3a" perceptual_roughness={0.6} />
        </Mesh3d>
        <Mesh3d transform={Transform.fromXyz(0, 0.6, 0)}>
          <Cuboid half_size={[0.45, 0.6, 0.45]} />
          <StandardMaterial base_color="#e8b04a" perceptual_roughness={0.35} metallic={0.2} />
        </Mesh3d>
      </Entity>
      <div style={{ position: "absolute", top: 80, width: "100%", textAlign: "center", fontFamily: "Inter", fontWeight: 800, fontSize: 72, color: "#ffffff" }}>
        Made to be seen
      </div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="turntable" component={Turntable} width={1080} height={1080} fps={30} durationInFrames={90} />);
