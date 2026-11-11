// raea cloud's smoke test against a running server (docs/cloud.feature CL12), with test/echo-agent.mjs as its agent:
//   node test/smoke.ts URL TOKEN
// smoke.sh starts the container and runs this. Each check prints one line; the exit code is the number that failed.
import { RaeaCloud, type Update } from "../src/index.ts"

const [url, token] = process.argv.slice(2)
if (!url || !token) throw new Error("usage: node test/smoke.ts URL TOKEN")
const cloud = new RaeaCloud({ url, token })
let failed = 0
const check = async (name: string, f: () => Promise<string>) => {
  const t0 = performance.now()
  try {
    console.log(`ok   ${name}: ${await f()} (${Math.round(performance.now() - t0)} ms)`)
  } catch (e) {
    failed++
    console.log(`FAIL ${name}: ${(e as Error).message}`)
  }
}
const status = async (path: string, init: RequestInit = {}) => (await fetch(url + path, init)).status
const expect = (cond: unknown, msg: string) => {
  if (!cond) throw new Error(msg)
}

await check("healthz needs no token", async () => {
  expect((await status("/healthz")) === 200, "not 200")
  return "200"
})
await check("no token, a wrong token: 401", async () => {
  const a = await status("/v1/health")
  const b = await status("/v1/health", { headers: { authorization: `Bearer ${"x".repeat(token.length)}` } })
  expect(a === 401 && b === 401, `${a}, ${b}`)
  return `${a}, ${b}`
})
let workspace = ""
await check("health", async () => {
  const h = await cloud.health()
  workspace = h.workspace
  expect(/^\d+\.\d+\.\d+/.test(h.raea), `raea ${h.raea}`)
  return `${h.raea}, agent ${h.agent}, installed ${h.agents.map((a) => `${a.id} ${a.version}`).join(", ") || "none"}, ${h.workspace}`
})
await check("files round trip", async () => {
  await cloud.write("smoke/a/b.txt", "hello cloud")
  const back = await cloud.text("smoke/a/b.txt")
  const ls = await cloud.list("smoke/a")
  expect(back === "hello cloud", `read back ${JSON.stringify(back)}`)
  expect(ls.some((e) => e.name === "b.txt" && e.bytes === 11), `listed ${JSON.stringify(ls)}`)
  return "written, read, listed"
})
await check("paths out of the workspace: 403", async () => {
  const auth = { authorization: `Bearer ${token}` }
  const got = [
    await status(`/v1/files?path=${encodeURIComponent("../../../etc/passwd")}`, { headers: auth }),
    await status(`/v1/files?path=${encodeURIComponent("/etc/passwd")}`, { headers: auth }),
    await status(`/v1/files?path=${encodeURIComponent("/tmp/x")}`, { method: "PUT", body: "x", headers: auth }),
    await status(`/v1/files?path=${encodeURIComponent("out-link/passwd")}`, { headers: auth }),
  ]
  expect(got.every((s) => s === 403), got.join(", "))
  return `.., absolute, PUT absolute, symlink out: ${got.join(", ")}`
})
await check("raea renders through /v1/run", async () => {
  let log = ""
  const init = await cloud.run(["init"], { cwd: "smoke/p", onOutput: (t) => (log += t) })
  expect(init === 0, `raea init exit ${init}: ${log.slice(-400)}`)
  const t0 = performance.now()
  const r = await cloud.run(["render", "edit.tsx", "--frame", "15", "-o", "card.png"], { cwd: "smoke/p", onOutput: (t) => (log += t) })
  expect(r === 0, `render exit ${r}: ${log.slice(-400)}`)
  const png = await cloud.read("smoke/p/card.png")
  expect(png[0] === 0x89 && png[1] === 0x50, "not a PNG")
  return `card.png ${png.length} bytes, render ${Math.round(performance.now() - t0)} ms`
})
await check("ACP turn with the stand-in agent", async () => {
  const s = await cloud.session({ cwd: "smoke" })
  const said: string[] = []
  const stop = await s.prompt("hello from the smoke test", (u: Update) => u.content?.text && said.push(u.content.text))
  s.close()
  const all = said.join(" | ")
  expect(stop === "end_turn", `stop ${stop}`)
  expect(said.includes("echo: hello from the smoke test"), all)
  expect(/raea: \d+\.\d+\.\d+/.test(all), `no raea in ${all}`)
  expect(all.includes(`cwd: ${workspace}`), `cwd not the workspace: ${all}`)
  expect(all.includes("token visible: no"), `the agent sees the token: ${all}`)
  expect(all.includes("permission: yes"), `permission not allowed once: ${all}`)
  return all
})
await check("ACP socket without a token: refused", async () => {
  const ws = new WebSocket(`${url.replace(/^http/, "ws")}/v1/acp`)
  const opened = await new Promise<boolean>((r) => ((ws.onopen = () => r(true)), (ws.onerror = () => r(false))))
  expect(!opened, "opened")
  return "refused"
})
console.log(failed ? `smoke: ${failed} FAILED` : "smoke: all passed")
process.exit(failed)
