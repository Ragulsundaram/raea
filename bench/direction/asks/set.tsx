// DR6's worked checks, the pieces they run on (docs/direction.feature DR6): an office (desks facing their chairs, a
// kettle on the counter, and an orb where a cheat puts one), a computer riding a swivel chair, a lyric over a singer,
// and a reveal held. Each composition beside this file is one of them as the notes ask, or a cheat that must fail.
import { AbsoluteFill, Sequence } from "raea";
import { Sphere, Text3D } from "raea/3d";
import { Camera3d } from "raea/camera";
import { DirectionalLight } from "raea/light";
import { Cuboid, Mesh3d } from "raea/mesh";
import { StandardMaterial } from "raea/pbr";
import { Quat, Transform } from "raea/transform";

const wood = "#8a5a3c";

function Box({ name, at, half, yaw = 0, color = wood }: { name: string; at: number[]; half: [number, number, number]; yaw?: number; color?: string }) {
  return (
    <Mesh3d name={name} transform={Transform.fromXyz(at[0], at[1], at[2]).withRotation(Quat.fromEuler("XYZ", 0, yaw, 0))}>
      <Cuboid half_size={half} />
      <StandardMaterial base_color={color} />
    </Mesh3d>
  );
}

/** An orb, as a note means one: what the Orb component draws, whatever its mesh is called. */
function Orb({ name = "orb", at, size = 1 }: { name?: string; at: number[]; size?: number }) {
  return (
    <Mesh3d name={name} transform={Transform.fromXyz(at[0], at[1], at[2]).withScale([size, size, size])}>
      <Sphere radius={0.25} />
      <StandardMaterial base_color="#9fd8ff" />
    </Mesh3d>
  );
}

/** Four desks in a row, each with its chair on its front (+z) side, and a counter with a kettle. `turned` turns one
 * desk round; `kettle: "desk"` sets the kettle on a desk; `desk` names the desks; `orb` puts an orb in. */
export function Office({ turned, kettle = "counter", desk = "desk", orb }: { turned?: number; kettle?: "counter" | "desk"; desk?: string; orb?: "one" | "renamed" | "tiny" }) {
  return (
    <AbsoluteFill style={{ background: "#202630" }}>
      <Camera3d transform={Transform.fromXyz(0, 6, 9).lookingAt([0, 0, 0])} />
      <DirectionalLight illuminance={9000} transform={Transform.fromXyz(3, 6, 5).lookingAt([0, 0, 0])} />
      {[-3, -1, 1, 3].map((x, i) => (
        <Box key={`d${i}`} name={`${desk}-${i + 1}`} at={[x, 0.375, 0]} half={[0.7, 0.375, 0.4]} yaw={turned === i + 1 ? Math.PI : 0} />
      ))}
      {[-3, -1, 1, 3].map((x, i) => (
        <Box key={`c${i}`} name={`chair-${i + 1}`} at={[x, 0.25, 0.9]} half={[0.25, 0.25, 0.25]} color="#333a44" />
      ))}
      <Box name="counter" at={[0, 0.45, -3]} half={[1.5, 0.45, 0.35]} color="#c9c2b6" />
      <Box name="kettle" at={kettle === "desk" ? [1, 0.875, 0] : [0.5, 1.025, -3]} half={[0.12, 0.125, 0.12]} color="#d33" />
      {orb === "one" && <Orb at={[4.5, 2.5, -2]} />}
      {orb === "renamed" && <Orb name="lamp-glass" at={[4.5, 2.5, -2]} />}
      {orb === "tiny" && <Orb at={[4.5, 2.5, -2]} size={0.0001} />}
    </AbsoluteFill>
  );
}

/** A point `[x, y, z]` of the chair's own space, turned by `yaw` about its seat at (0, 0.5, 0). */
const on = (yaw: number, [x, y, z]: number[]) => Transform.fromXyz(x * Math.cos(yaw) + z * Math.sin(yaw), 0.5 + y, -x * Math.sin(yaw) + z * Math.cos(yaw)).withRotation(Quat.fromEuler("XYZ", 0, yaw, 0));

/** A swivel chair turned by `yaw`, its back behind, and a computer on its seat at `at` in the chair's space, turned
 * with it; `left` leaves the computer where it was on frame 0 (yaw 0.3) as the chair turns; `rider` names it. */
export function Ride({ yaw, at, left, rider = "computer" }: { yaw: number; at: number[]; left?: boolean; rider?: string }) {
  return (
    <AbsoluteFill style={{ background: "#1d2430" }}>
      <Camera3d transform={Transform.fromXyz(2.5, 2, 4).lookingAt([0, 0.8, 0])} />
      <DirectionalLight illuminance={9000} transform={Transform.fromXyz(2, 4, 6).lookingAt([0, 0, 0])} />
      <Mesh3d name="chair" transform={on(yaw, [0, 0, 0])}>
        <Cuboid half_size={[0.3, 0.05, 0.3]} />
        <StandardMaterial base_color={wood} />
      </Mesh3d>
      <Mesh3d name="chair-back" transform={on(yaw, [0, 0.45, -0.25])}>
        <Cuboid half_size={[0.3, 0.4, 0.05]} />
        <StandardMaterial base_color={wood} />
      </Mesh3d>
      <Mesh3d name={rider} transform={on(left ? 0.3 : yaw, at)}>
        <Cuboid half_size={[0.2, 0.15, 0.15]} />
        <StandardMaterial base_color="#3fa7ff" />
      </Mesh3d>
    </AbsoluteFill>
  );
}

/** The singer: a cutout standing in for a person, drawn after the line so it is in front of it. */
function Singer() {
  return <div style={{ position: "absolute", left: 220, top: 250, width: 180, height: 710, borderRadius: "90px 90px 0 0", background: "#2b2b2b" }} />;
}

/** The lyric over the singer as flat type; `deep` sets it as 3D type a hair deep, facing the camera; `bare` leaves
 * the singer out; `words` changes the line. */
export function Lyric({ deep, bare, words = "OUT OF THE GREY" }: { deep?: boolean; bare?: boolean; words?: string }) {
  if (deep) {
    return (
      <AbsoluteFill style={{ background: "#16202e" }}>
        <Camera3d transform={Transform.fromXyz(0, 0, 6).lookingAt([0, 0, 0])} />
        <DirectionalLight illuminance={9000} transform={Transform.fromXyz(0, 2, 6).lookingAt([0, 0, 0])} />
        <Mesh3d name="line" transform={Transform.fromXyz(0, 1, 0)}>
          <Text3D size={0.25} depth={0.002}>{words}</Text3D>
          <StandardMaterial base_color="#ffffff" />
        </Mesh3d>
      </AbsoluteFill>
    );
  }
  return (
    <AbsoluteFill style={{ background: "#3a4a5e" }}>
      <div style={{ position: "absolute", left: 60, top: 300, width: 420, fontSize: 64, fontWeight: 800, color: "#ffffff", lineHeight: 1.1 }}>{words}</div>
      {!bare && <Singer />}
    </AbsoluteFill>
  );
}

/** A cut: a lead-in, then the reveal for `seconds`, named `name`. */
export function Hold({ seconds, name = "reveal" }: { seconds: number; name?: string }) {
  return (
    <AbsoluteFill style={{ background: "#111" }}>
      <Sequence from={0} durationInFrames={30} name="lead-in">
        <AbsoluteFill style={{ background: "#2a3340" }} />
      </Sequence>
      <Sequence from={30} durationInFrames={Math.round(seconds * 30)} name={name}>
        <AbsoluteFill style={{ background: "#c8a24a" }} />
      </Sequence>
    </AbsoluteFill>
  );
}
