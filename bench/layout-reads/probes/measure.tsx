// LR6's reads as a probe (docs/layout-reads.feature): a 2D context's measureText and its image data. Each measure
// places a word (its left or top at the value read), so the harness's 1 px box check holds it to Chrome's; the pixels
// read back are printed as words, which must match exactly.
import { AbsoluteFill } from "raea";
import { loadFont } from "raea/text";
import { useLayoutEffect, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 360, fps: 30, durationInFrames: 3 };
export const props = {};

type Read = { w: number; inkL: number; inkR: number; asc: number; desc: number; fasc: number; pixels: string; put: string };

function read(): Read {
  const c = new OffscreenCanvas(64, 64);
  const ctx = c.getContext("2d")!;
  ctx.font = `48px ${fontFamily}`;
  const hello = ctx.measureText("Hello, measured");
  const g = ctx.measureText("g");
  ctx.textAlign = "center";
  const mid = ctx.measureText("Wide");
  ctx.textAlign = "left";
  // what was drawn, read back: a red square on clear, the middle of it and a corner outside it
  ctx.fillStyle = "#f00";
  ctx.fillRect(8, 8, 16, 16);
  const px = (x: number, y: number) => [...ctx.getImageData(x, y, 1, 1).data].join(" ");
  // putImageData writes its pixels as they are: half-transparent green, no blending with the red under it
  const img = ctx.createImageData(2, 2);
  for (let i = 0; i < img.data.length; i += 4) img.data.set([0, 255, 0, 128], i);
  ctx.globalAlpha = 0.25;
  ctx.putImageData(img, 10, 10);
  return {
    w: hello.width,
    inkL: mid.actualBoundingBoxLeft,
    inkR: mid.actualBoundingBoxRight,
    asc: hello.actualBoundingBoxAscent,
    desc: g.actualBoundingBoxDescent,
    fasc: hello.fontBoundingBoxAscent,
    pixels: `${px(16, 16)} and ${px(40, 40)}`,
    put: px(11, 11),
  };
}

export function Comp() {
  const [r, setR] = useState<Read | null>(null);
  useLayoutEffect(() => setR(read()), []);
  if (!r) return <AbsoluteFill style={{ background: "#fff" }} />;
  const at = (left: number, top: number, word: string) => (
    <div style={{ position: "absolute", left, top, fontSize: 16, lineHeight: "20px" }}>{word}</div>
  );
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily }}>
      {at(r.w, 10, "width")}
      {at(320 - r.inkL, 40, "inkleft")}
      {at(320 + r.inkR, 70, "inkright")}
      {at(20, 100 + r.asc, "ascent")}
      {at(20 + r.desc * 10, 160, "descent")}
      {at(20, 180 + r.fasc, "fontascent")}
      {at(20, 260, `red ${r.pixels}`)}
      {at(20, 290, `put ${r.put}`)}
    </AbsoluteFill>
  );
}
