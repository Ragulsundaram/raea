// 120 small filtered boxes, each showing 60 px of a child that covers the frame from any of them, cut four ways, 30
// boxes each: by a clipping box inside; by one positioned with a z-index (the filtered box's stacking context paints
// it); by the filtered box's own clip, the child positioned with a z-index; and by a `contain: paint` box inside.
// 120 grey 60 px tiles on white. A filter's layer reaches over what its element draws, and stops at a clip in
// the way: made from the element's overflow, which reaches through clips, each of these layers would be the size of
// the frame, and 30 of them are more than Vello's image atlas holds, of either kind. bench/filter-overflow/check.sh
import { AbsoluteFill, Composition, registerRoot } from "raea";
function O() {
  return (
    <AbsoluteFill style={{ background: "#fff" }}>
      {Array.from({ length: 120 }, (_, i) => {
        const kind = Math.floor(i / 30);
        const cover = <div style={{ position: "relative", left: -2000, top: -1500, width: 4000, height: 3000, background: "#008000", ...(kind === 2 ? { zIndex: 1 } : {}) }} />;
        const inner = [{ overflow: "hidden" }, { overflow: "hidden", position: "relative", zIndex: 1 }, null, { contain: "paint" }][kind];
        return (
          <div key={i} style={{ position: "absolute", left: 30 + (i % 12) * 155, top: 20 + Math.floor(i / 12) * 105, width: 60, height: 60, filter: "grayscale(1)", ...(kind === 2 ? { overflow: "hidden" } : {}) }}>
            {inner ? <div style={{ width: 60, height: 60, ...inner }}>{cover}</div> : cover}
          </div>
        );
      })}
    </AbsoluteFill>
  );
}
registerRoot(() => <Composition id="o" component={O} width={1920} height={1080} fps={30} durationInFrames={1} />);
