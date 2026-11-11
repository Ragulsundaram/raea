# raea cloud

raea and an ACP agent on a server of your own. One image holds raea's Linux release, the agents you choose and a
small server; you deploy it wherever you run containers, and talk to it over HTTP and ACP. raea doesn't host it: the
server, the GPU and the agent's subscription are yours. Spec: [docs/cloud.feature](../../docs/cloud.feature).

```
your code ── RaeaCloud (src/) ──▶ :8080  raea-cloud server ──▶ raea (render, play, check, ...)
             HTTPS + one token           /v1/acp  (WebSocket) ──▶ the agent, over ACP, in ~/work
                                         /v1/files, /v1/run, /v1/login, /v1/health
```

## Run it

```bash
cd sdk/cloud
docker build --platform linux/amd64 -f image/Dockerfile \
  --build-arg RAEA_VERSION=0.1.1 --build-arg AGENTS="kimi-code" -t raea-cloud .
export RAEA_CLOUD_TOKEN=$(openssl rand -hex 32)
docker run -d -p 127.0.0.1:8080:8080 -e RAEA_CLOUD_TOKEN -v raea-home:/home/raea raea-cloud
curl -H "authorization: Bearer $RAEA_CLOUD_TOKEN" localhost:8080/v1/health
```

`AGENTS` takes any ids the [ACP registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
lists, several at once: `kimi-code`, `claude-acp`, `codex-acp`, `gemini`, `opencode`, `goose`, ... Each is installed
from its registry entry (npm at the registry's version, or its Linux archive checked by sha256), and raea's skill is
installed for it on every start. `RAEA_AGENT` picks the default; a socket can name another (`?agent=`).
`RAEA_AGENT_CMD` runs an agent the registry doesn't list, by its command line.

Signing the agent in:

- **Kimi Code** signs in by a device code: `POST /v1/login?agent=kimi-code` (or `cloud.login()`) returns a link and
  a code to approve once. The sign-in is kept in the home folder.
- **Claude, Codex, Gemini** take their key from the container's environment: `ANTHROPIC_API_KEY` (or
  `CLAUDE_CODE_OAUTH_TOKEN`), `OPENAI_API_KEY`, `GEMINI_API_KEY`.

## Use it

```ts
import { RaeaCloud } from "@raea/cloud" // sdk/cloud/src

const cloud = new RaeaCloud({ url: "https://raea.example.com", token: process.env.RAEA_CLOUD_TOKEN! })

// raea on its own
await cloud.run(["init"], { cwd: "card" })
await cloud.run(["render", "edit.tsx", "-o", "card.mp4"], { cwd: "card", onOutput: (t) => process.stdout.write(t) })
const mp4 = await cloud.read("card/card.mp4")

// or an agent that uses raea, over ACP
const s = await cloud.session()
const stop = await s.prompt("Make card/edit.tsx a 3 s title card that says hello, and render it", (u) => {
  if (u.sessionUpdate === "agent_message_chunk") process.stdout.write(u.content?.text ?? "")
})
s.close()
```

The client needs only `fetch` and `WebSocket`: Node 22+, Bun, Deno or a browser. Permission requests are allowed
once by default (the container is the sandbox); pass `onPermission` to decide them yourself.

| Route | |
|---|---|
| `GET /healthz` | `ok`, without the token, for health checks |
| `GET /v1/health` | raea's version, the default agent, the installed agents, the workspace |
| `GET /v1/files?path=P` | a file in the workspace, or a folder's entries as JSON |
| `PUT /v1/files?path=P` | writes the body there, making its folders |
| `POST /v1/run` `{args, cwd}` | `raea ARGS` in the workspace; NDJSON `{out}`, `{err}`, then `{exit}` |
| `POST /v1/login?agent=ID` | an agent's terminal sign-in: `{url, code, state}`; `GET` says where it is |
| `WS /v1/acp?agent=ID` | one agent per socket; each text frame is one ACP JSON-RPC message |

Every route but `/healthz` needs `Authorization: Bearer TOKEN` (or `?token=`, for a browser's WebSocket). Paths
outside the workspace are refused, symlinks out of it too. The agent and raea never see the token.

## Deploy it

| Where | File | The home folder lives on | Checked |
|---|---|---|---|
| Docker, Compose | `deploy/compose.yaml` | a named volume | end to end (test/smoke.sh) |
| Compose with an NVIDIA GPU | `+ deploy/compose.gpu.yaml` | a named volume | not yet on a GPU host |
| Fly Machines | `deploy/fly.toml` | a Fly volume; the Machine stops when idle | config only |
| Kubernetes | `deploy/k8s.yaml` | a PersistentVolumeClaim | config only |
| Cloudflare Containers | `deploy/cloudflare/` | R2, through `RAEA_CLOUD_STATE` | config only |
| Any Linux VM | `docker run`, as above | a named volume | as Docker |

Where the disk doesn't outlive the container, set `RAEA_CLOUD_STATE` to an S3-compatible object (R2, S3) with
`RAEA_CLOUD_STATE_KEY` and `RAEA_CLOUD_STATE_SECRET`: the home folder is restored from it on start and saved to it
when the container stops (image/state.sh).

Each file says how to deploy with it at its top.

## Renders: CPU or GPU

raea renders through Vulkan. In a container without a GPU that's Mesa's llvmpipe on the CPU: right for stills, short
clips and checks, and faster with more cores. For long or heavy renders give the container an NVIDIA GPU
(compose.gpu.yaml, or `nvidia.com/gpu` on Kubernetes) with the graphics capability, so the toolkit mounts NVIDIA's
Vulkan driver.

## Known limits

- raea ships for Linux on x86_64 only, so the image is linux/amd64; an arm64 machine (Graviton, Ampere, Apple
  Silicon's Docker) runs it emulated and slowly.
- One server is one workspace, for one person or one team that trusts each other: everyone with the token has the
  agent's whole sandbox.
- Agents installed with uvx (Python) aren't supported by the image; add Python in your own image or run them by
  `RAEA_AGENT_CMD`.

## Test it

`test/smoke.sh 0.1.1` builds the image, starts it with a stand-in agent (test/echo-agent.mjs) and checks the token,
the path rules, a render through `/v1/run` and an ACP turn.
