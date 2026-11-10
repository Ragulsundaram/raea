// A filter takes in all its element draws, what its descendants draw outside its box included. Top: a wrapper of no
// size with a blur, around two absolutely positioned squares; and a 60 px box with grayscale(1) around a red square
// whose own drop shadow falls 100 px to its right. Below, a 60 px box with grayscale(1) holding a 200x100 green
// child, three ways: as it is (all of the child shows), clipping its overflow (60 px of it), and clipping x alone
// (60 px wide, all 100 px tall). bench/filter-overflow/check.sh
import { AbsoluteFill, Composition, registerRoot } from "raea";
const square = (left: number, background: string) => <div style={{ position: "absolute", left, top: 0, width: 120, height: 120, background }} />;
const box = (left: number, clip: object) => (
  <div style={{ position: "absolute", left, top: 220, width: 60, height: 60, filter: "grayscale(1)", ...clip }}>
    <div style={{ width: 200, height: 100, background: "#008000" }} />
  </div>
);
function O() {
  return (
    <AbsoluteFill style={{ background: "#fff" }}>
      <div style={{ position: "absolute", left: 40, top: 40, filter: "blur(4px)" }}>{square(0, "#f00")}{square(140, "#00f")}</div>
      <div style={{ position: "absolute", left: 400, top: 60, width: 60, height: 60, filter: "grayscale(1)" }}>
        <div style={{ width: 60, height: 60, background: "#f00", filter: "drop-shadow(100px 0 0 #f00)" }} />
      </div>
      {box(40, {})}
      {box(340, { overflow: "hidden" })}
      {box(480, { overflowX: "clip", overflowY: "visible" })}
    </AbsoluteFill>
  );
}
registerRoot(() => <Composition id="o" component={O} width={640} height={360} fps={30} durationInFrames={1} />);
