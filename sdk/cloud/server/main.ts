// raea cloud's server (docs/cloud.feature CL4-CL8): raea and an ACP agent behind one port and one token.
//
//   GET  /healthz                       "ok", no token (platform health checks)
//   GET  /v1/health                     {raea, agent, agents, workspace}
//   GET  /v1/files?path=P               a file in the workspace, or a folder's entries as JSON
//   PUT  /v1/files?path=P               writes the body to P, making its folders
//   POST /v1/run {args, cwd?}           `raea ARGS` in the workspace: NDJSON {out} and {err} lines, then {exit}
//   POST /v1/login?agent=ID             an agent's terminal sign-in: {url, code, state, output}; GET says where it is
//   WS   /v1/acp?agent=ID               one agent process per socket: a text frame is one ACP message to its stdin,
//                                       each line it prints is one frame back
//
// Every route but /healthz needs RAEA_CLOUD_TOKEN: `Authorization: Bearer TOKEN`, or ?token= (a browser's WebSocket
// can't set headers). The agent and raea get the container's environment without the token or the state keys.

import { execFileSync, spawn } from "node:child_process"
import { timingSafeEqual } from "node:crypto"
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs"
import { createServer, type IncomingMessage, type ServerResponse } from "node:http"
import path from "node:path"
import { WebSocketServer } from "ws"
import { installed, spawnFor } from "./agents.ts"
import { login, loginState } from "./login.ts"
import { Outside, resolve, WORK } from "./workspace.ts"

const TOKEN = process.env.RAEA_CLOUD_TOKEN ?? ""
if (TOKEN.length < 16) {
  console.error("raea-cloud: set RAEA_CLOUD_TOKEN to a secret of 16 characters or more (openssl rand -hex 32); every request must carry it")
  process.exit(1)
}
const PORT = Number(process.env.PORT ?? 8080)
const RAEA = process.env.RAEA_BIN ?? "raea"

/** The environment raea and the agents run with: the container's, less this server's own secrets. */
const childEnv = (): NodeJS.ProcessEnv => {
  const env = { ...process.env }
  for (const k of Object.keys(env)) if (k === "RAEA_CLOUD_TOKEN" || k.startsWith("RAEA_CLOUD_STATE")) delete env[k]
  return env
}

let version: string | undefined
const raeaVersion = () => {
  try {
    version ??= execFileSync(RAEA, ["--version"], { encoding: "utf8", timeout: 20_000, env: childEnv() }).trim()
  } catch {
    version = "unknown"
  }
  return version
}

function authorized(req: IncomingMessage, url: URL): boolean {
  const given = /^Bearer (.+)$/.exec(req.headers.authorization ?? "")?.[1] ?? url.searchParams.get("token") ?? ""
  const a = Buffer.from(given)
  const b = Buffer.from(TOKEN)
  return a.length === b.length && timingSafeEqual(a, b)
}

const json = (res: ServerResponse, v: unknown, status = 200) => res.writeHead(status, { "content-type": "application/json" }).end(JSON.stringify(v))

async function body(req: IncomingMessage): Promise<Buffer> {
  const parts: Buffer[] = []
  for await (const c of req) parts.push(c as Buffer)
  return Buffer.concat(parts)
}

function files(req: IncomingMessage, res: ServerResponse, url: URL) {
  const p = resolve(url.searchParams.get("path") ?? ".")
  if (req.method === "PUT")
    return body(req).then((b) => {
      mkdirSync(path.dirname(p), { recursive: true })
      writeFileSync(p, b)
      json(res, { path: path.relative(WORK, p), bytes: b.length })
    })
  let st
  try {
    st = statSync(p)
  } catch {
    return json(res, { error: `${url.searchParams.get("path")} doesn't exist` }, 404)
  }
  if (st.isDirectory()) return json(res, readdirSync(p, { withFileTypes: true }).map((d) => ({ name: d.name, dir: d.isDirectory(), bytes: d.isFile() ? statSync(path.join(p, d.name)).size : undefined })))
  res.writeHead(200, { "content-type": "application/octet-stream", "content-length": st.size }).end(readFileSync(p))
}

async function run(req: IncomingMessage, res: ServerResponse) {
  const { args, cwd } = JSON.parse((await body(req)).toString() || "{}") as { args?: unknown; cwd?: string }
  if (!Array.isArray(args) || !args.every((a) => typeof a === "string")) return json(res, { error: "args must be an array of strings: raea's own arguments" }, 400)
  const dir = resolve(cwd ?? ".")
  mkdirSync(dir, { recursive: true })
  res.writeHead(200, { "content-type": "application/x-ndjson" })
  const child = spawn(RAEA, args, { cwd: dir, env: childEnv(), stdio: ["ignore", "pipe", "pipe"] })
  const line = (v: unknown) => res.write(JSON.stringify(v) + "\n")
  child.stdout.setEncoding("utf8").on("data", (out: string) => line({ out }))
  child.stderr.setEncoding("utf8").on("data", (err: string) => line({ err }))
  child.on("error", (e) => (line({ err: String(e) }), res.end(JSON.stringify({ exit: 127 }) + "\n")))
  child.on("close", (code, signal) => res.end(JSON.stringify({ exit: code ?? 128, signal: signal ?? undefined }) + "\n"))
  // the caller went away: the run stops with it
  res.on("close", () => child.exitCode === null && child.kill("SIGTERM"))
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", "http://raea-cloud")
  if (url.pathname === "/healthz") return res.writeHead(200, { "content-type": "text/plain" }).end("ok")
  if (!authorized(req, url)) return json(res, { error: "unauthorized" }, 401)
  try {
    if (url.pathname === "/v1/health" && req.method === "GET") {
      const def = (() => {
        try {
          return spawnFor().id
        } catch {
          return null
        }
      })()
      return json(res, { raea: raeaVersion(), agent: def, agents: installed().map((a) => ({ id: a.id, version: a.version })), workspace: WORK })
    }
    if (url.pathname === "/v1/files" && (req.method === "GET" || req.method === "PUT")) return await files(req, res, url)
    if (url.pathname === "/v1/run" && req.method === "POST") return await run(req, res)
    if (url.pathname === "/v1/login") {
      const a = spawnFor(url.searchParams.get("agent") ?? undefined)
      return json(res, req.method === "POST" ? await login(a, childEnv()) : loginState(a.id))
    }
    json(res, { error: `no route ${req.method} ${url.pathname}` }, 404)
  } catch (e) {
    if (e instanceof Outside) return json(res, { error: e.message }, 403)
    if (!res.headersSent) json(res, { error: (e as Error).message }, 400)
  }
})

// the WebSocket upgrade checks the token before a socket opens
const wss = new WebSocketServer({ noServer: true })
server.on("upgrade", (req, socket, head) => {
  const url = new URL(req.url ?? "/", "http://raea-cloud")
  if (url.pathname !== "/v1/acp" || !authorized(req, url)) return socket.end("HTTP/1.1 401 Unauthorized\r\n\r\n")
  let a
  try {
    a = spawnFor(url.searchParams.get("agent") ?? undefined)
  } catch (e) {
    return socket.end(`HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain\r\n\r\n${(e as Error).message}`)
  }
  const agent = a
  wss.handleUpgrade(req, socket, head, (ws) => {
    const child = spawn(agent.command, agent.args, { cwd: WORK, env: { ...childEnv(), ...agent.env }, stdio: ["pipe", "pipe", "inherit"] })
    let buf = ""
    // ACP messages are newline-delimited JSON: split on \n only
    child.stdout.setEncoding("utf8").on("data", (c: string) => {
      buf += c
      for (let i; (i = buf.indexOf("\n")) >= 0; buf = buf.slice(i + 1)) if (i > 0) ws.send(buf.slice(0, i))
    })
    ws.on("message", (m) => child.stdin.write(m.toString() + "\n"))
    ws.on("close", () => child.exitCode === null && child.kill("SIGTERM"))
    child.on("error", (e) => ws.close(1011, `couldn't start ${agent.id}: ${e.message}`.slice(0, 120)))
    child.on("exit", (code) => ws.close(1011, `${agent.id} exited ${code}`))
  })
})

mkdirSync(WORK, { recursive: true })
server.listen(PORT, () => console.log(`raea-cloud: :${PORT}, workspace ${WORK}, agents ${installed().map((a) => a.id).join(", ") || "none"}, raea ${raeaVersion()}`))
