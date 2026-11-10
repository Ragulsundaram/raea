// check: frames 0,12,40
import { AbsoluteFill, Composition, registerRoot, spring, useCurrentFrame, useVideoConfig } from "raea";

const WORDS = ["Make", "what", "you", "imagine."];

// Each word springs up into place, four frames after the one before it.
function Title() {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return (
    <AbsoluteFill style={{ background: "#0f1115", justifyContent: "center", alignItems: "center", flexDirection: "row", gap: 28 }}>
      {WORDS.map((word, i) => {
        const p = spring({ frame, fps, delay: i * 4, config: { damping: 14, stiffness: 160 } });
        return (
          <span key={word} style={{ color: "#ffffff", fontSize: 110, fontWeight: 700, opacity: p, transform: `translateY(${(1 - p) * 80}px)` }}>
            {word}
          </span>
        );
      })}
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="title" component={Title} width={1920} height={1080} fps={30} durationInFrames={60} />);
