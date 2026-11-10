// check: frames 0,30,89
import { AbsoluteFill, Composition, Sequence, interpolate, registerRoot, useCurrentFrame } from "raea";

// One card: its text fades in over the first 12 frames of its Sequence.
function Card({ text }: { text: string }) {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 12], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ background: "#000", color: "#fff", justifyContent: "center", alignItems: "center", fontSize: 120 }}>
      <span style={{ opacity }}>{text}</span>
    </AbsoluteFill>
  );
}

// Three seconds at 30 fps, vertical: "Hello" for the first half, "world" for the second.
registerRoot(() => (
  <Composition
    id="hello"
    width={1080}
    height={1920}
    fps={30}
    durationInFrames={90}
    component={() => (
      <>
        <Sequence from={0} durationInFrames={45}>
          <Card text="Hello" />
        </Sequence>
        <Sequence from={45} durationInFrames={45}>
          <Card text="world" />
        </Sequence>
      </>
    )}
  />
));
