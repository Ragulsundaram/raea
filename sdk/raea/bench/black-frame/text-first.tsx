// The black-frame warning's text case (#96): near-black words come before anything with a colour of its own, so the
// warning's search for what the page shows reads text nodes on its way to the red box. check.sh renders it with every
// draw black (RAEA_BLACK_FAULT=3): it must warn, naming the box, not panic on a text node (text-build did, 11-12).
import { AbsoluteFill, Composition, registerRoot } from "raea";

function Shot() {
  return (
    <AbsoluteFill style={{ color: "#050505", fontSize: 40, padding: 40 }}>
      dark words first
      <div style={{ width: 200, height: 100, background: "#c03030" }} />
    </AbsoluteFill>
  );
}
registerRoot(() => <Composition id="main" component={Shot} durationInFrames={2} fps={30} width={640} height={360} />);
