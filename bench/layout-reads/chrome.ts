// The Chrome side's frames (check.sh): the pinned Chrome for Testing, driven by Playwright, renders the composition's
// page at each frame in real time, so requestAnimationFrame and ResizeObserver run as they do in a browser (a headless
// --dump-dom under --virtual-time-budget runs neither: no rendering step ever comes). Each frame's page is loaded with
// `#FRAME,dump`; once boxes.ts's runs are on it (the page adds them a second after its fonts are ready), they're written
// to OUT/chrome-FRAME.json and the page to OUT/chrome-FRAME.png.
//   bun bench/layout-reads/chrome.ts CHROME URL WIDTH HEIGHT OUT FRAME...

import { writeFileSync } from "node:fs";
import { chromium } from "../../apps/app/node_modules/@playwright/test";

const [exe, url, w, h, out, ...frames] = process.argv.slice(2);
if (!exe || !url || !out || !frames.length) {
  console.error("usage: chrome.ts CHROME URL WIDTH HEIGHT OUT FRAME...");
  process.exit(2);
}
const browser = await chromium.launch({ executablePath: exe, args: ["--hide-scrollbars"] });
try {
  for (const f of frames) {
    const page = await browser.newPage({ viewport: { width: Number(w), height: Number(h) } });
    await page.goto(`${url}#${f},dump`);
    const runs = await page.waitForSelector("#lr-runs", { state: "attached", timeout: 30000 }).then((s) => s.textContent());
    writeFileSync(`${out}/chrome-${f}.json`, runs ?? "");
    await page.screenshot({ path: `${out}/chrome-${f}.png` });
    await page.close();
  }
} finally {
  await browser.close();
}
