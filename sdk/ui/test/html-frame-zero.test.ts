// Frame 0 of an HTML page applies its GSAP timeline's time-0 sets (issue #79). A paused timeline's playhead starts
// at 0, so seeking it to 0 was a no-op and frame 0 showed the page's raw CSS. Uses GSAP from jsdelivr, as
// bench/html-png does (html-run caches it under ~/Library/Caches/raea).
import { expect, test } from "bun:test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

test("frame 0 shows the timeline's sets at 0 (#79)", async () => {
  const dir = mkdtempSync(join(tmpdir(), "raea-html-frame0-"));
  writeFileSync(
    join(dir, "index.html"),
    `<!doctype html><html><head><script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script></head><body>
<div data-composition-id="main" data-start="0" data-duration="2" data-width="640" data-height="360"><div id="a">A</div><div id="b">B</div></div>
<script>window.__timelines = window.__timelines || {}; const tl = gsap.timeline({ paused: true });
tl.set("#a", { opacity: 0 }, 0); tl.set("#b", { x: 200 }, 0); tl.to("#a", { opacity: 1, duration: 0.5 }, 1);
window.__timelines.main = tl;</script></body></html>`,
  );
  const run = Bun.spawn(["bun", join(import.meta.dir, "html-run.ts"), dir, "0"], { stdout: "pipe", stderr: "pipe" });
  const out = await new Response(run.stdout).text();
  await run.exited;
  expect(out).toMatch(/id=a \{"opacity":0\}/);
  expect(out).toMatch(/id=b \{"transform":"translate\(200px, 0px\)"\}/);
}, 30_000);
