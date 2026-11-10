// N layers the size of the frame, each holding one white tile in a cell of its own, on black: a frame's mean luma
// counts the tiles drawn. An element with a perspective transform is drawn apart and placed as an image the size of
// its box, new in every frame, in Vello's image atlas. The perspective here is slight (the far corner moves under
// 3 px) and changes with the frame, so each frame is painted anew, the layers land a pixel or two apart in size, and
// no tile leaves its cell. bench/layer-atlas/check.sh
import { AbsoluteFill, Composition, useCurrentFrame } from "raea";

const W = 1920, H = 1080, COLS = 8;

function Planes({ n }: { n: number }) {
  const f = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      {Array.from({ length: n }, (_, i) => {
        // matrix3d's m14 and m24: w = 1 + a x + b y
        const a = (1e-7 * (1 + ((i + f) % 5))).toFixed(9), b = (1e-7 * (1 + ((2 * i + f) % 3))).toFixed(9);
        return (
          <div key={i} style={{ position: "absolute", left: 0, top: 0, width: W, height: H, transformOrigin: "0 0", transform: `matrix3d(1,0,0,${a},0,1,0,${b},0,0,1,0,0,0,0,1)` }}>
            <div style={{ position: "absolute", left: 40 + (i % COLS) * 230, top: 40 + Math.floor(i / COLS) * 200, width: 160, height: 100, background: "#fff" }} />
          </div>
        );
      })}
    </AbsoluteFill>
  );
}

export const planes = (n: number) => () => <Composition id="planes" component={() => <Planes n={n} />} width={W} height={H} fps={24} durationInFrames={48} />;
