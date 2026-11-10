// transform-box: view-box (an SVG element's default) under a viewBox that doesn't start at 0 0: the reference box is the
// viewBox's size at the origin of user space, so a 0 0 origin is user 0 0 and 50% 50% is half the viewBox's size.
// Chrome 157 draws the black square at 100,50 to 220,170, the green at 60,160 to 100,200 and the red at 70,90 to
// 110,130. bench/svg-css-transform/check.sh
import { AbsoluteFill, Composition, registerRoot } from "raea";
function O() {
  return (
    <AbsoluteFill style={{ background: "#fff" }}>
      <svg width={300} height={300} viewBox="100 50 300 300" style={{ position: "absolute", left: 0, top: 0 }}>
        <rect x={100} y={50} width={40} height={40} fill="#008000" style={{ transformOrigin: "50% 50%", transform: "rotate(180deg)" }} />
        <g transform="scale(2)"><rect x={100} y={50} width={60} height={60} fill="#000" /></g>
        <svg x={150} y={100} width={100} height={100} viewBox="10 20 50 50">
          <rect x={10} y={20} width={10} height={10} fill="#f00" transform="scale(2)" />
        </svg>
      </svg>
    </AbsoluteFill>
  );
}
registerRoot(() => <Composition id="o" component={O} width={300} height={300} fps={30} durationInFrames={1} />);
