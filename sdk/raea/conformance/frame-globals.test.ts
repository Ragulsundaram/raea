// requestAnimationFrame and Image where a composition's runtime has none (#56): callbacks run once per frame, given the
// frame's time, and a preload's load fires.
import { expect, test } from "bun:test";

const g = globalThis as any;
const had = { raf: g.requestAnimationFrame, caf: g.cancelAnimationFrame, Image: g.Image };
delete g.requestAnimationFrame;
delete g.cancelAnimationFrame;
delete g.Image;
const { frameCallbacks } = await import("../src/frame-globals");
const ours = { raf: g.requestAnimationFrame, caf: g.cancelAnimationFrame, Image: g.Image };
Object.assign(g, { requestAnimationFrame: had.raf, cancelAnimationFrame: had.caf, Image: had.Image });

test("a frame callback runs once, at the next frame, with that frame's time", () => {
  const got: number[] = [];
  ours.raf((t: number) => got.push(t));
  const gone = ours.raf(() => got.push(-1));
  ours.caf(gone);
  frameCallbacks(3, 30);
  frameCallbacks(3, 30);
  frameCallbacks(4, 30);
  expect(got).toEqual([100]);
});

test("a callback asked for while one runs waits for the frame after", () => {
  const got: number[] = [];
  ours.raf(() => ours.raf((t: number) => got.push(t)));
  frameCallbacks(10, 25);
  expect(got).toEqual([]);
  frameCallbacks(11, 25);
  expect(got).toEqual([440]);
});

test("an Image's load fires once its src is set", async () => {
  const img = new ours.Image();
  const seen: string[] = [];
  img.onload = () => seen.push("onload");
  img.addEventListener("load", () => seen.push("listener"));
  img.src = "/media/x.png";
  expect(img.complete).toBe(false);
  await Promise.resolve();
  expect([seen, img.complete, img.src]).toEqual([["onload", "listener"], true, "/media/x.png"]);
});
