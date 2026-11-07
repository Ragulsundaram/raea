// Shoots each parity case drawn by three.js in headless Chrome (docs/r3f.feature R3F8): the lab's #/r3f page, for
// `bun tools/shoot.ts BASE_URL|serve OUT_DIR [dir-case ...]` (every case in test/parity/fixtures.tsx by default);
// `serve` serves the page itself (tools/serve.ts).
import { resolve } from "node:path";
import { openChrome } from "./chrome";
import { serve } from "./serve";
const [baseArg, out, ...only] = process.argv.slice(2);
if (!baseArg || !out) throw new Error("usage: bun tools/shoot.ts BASE_URL|serve OUT_DIR [in-basic ...]");
const served = baseArg === "serve" ? await serve() : undefined;
const base = served?.url ?? baseArg;
const cases = only.length ? only : ["in-basic", "in-lights", "in-things", "in-catch", "out-basic", "out-tony", "out-lights", "out-bloom", "out-grade", "out-lens", "out-catch", "src-basic", "src-tony", "src-lights", "src-bloom", "src-grade", "src-lens", "src-catch"];
let failed = 0;
for (const c of cases) {
  const [dir, name] = c.split("-");
  const page = await openChrome(`${base}/#/r3f?dir=${dir}&case=${name}`, 640, 400, { scale: 1, mobile: false });
  try {
    let note = "";
    for (let i = 0; i < 300 && note !== "drawn" && !note.startsWith("error"); i++) {
      await Bun.sleep(100);
      note = String(await page.value(`[...document.querySelectorAll(".font-mono")].map(e=>e.textContent).join("")`));
    }
    await page.shot(resolve(out, `${c}-three.png`));
    const bad = page.problems.filter((p: string) => !/favicon/.test(p));
    if (process.env.LOGS) console.log(page.logs.join("\n"));
    console.log(`${c}: ${note}${bad.length ? `; problems: ${bad.join(" | ")}` : ""}`);
    if (note !== "drawn") failed++;
  } finally {
    page.close();
  }
}
served?.stop();
process.exit(failed ? 1 : 0);
