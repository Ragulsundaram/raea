@spec
Feature: raea cloud: raea and an ACP agent on a server of your own (sdk/cloud)
  Shane, 2026-11-10: "getting the kpop concept and its docker image up and running as a true module called cloud and
  would support the many configurations we could set up for deployment of raea engine to the cloud with acp agent.
  this is more of a developer thing for now".
  kpop proved the idea on one Fly Sprite: an agent in the cloud next to raea, spoken to over ACP through a small relay,
  with a TypeScript client. The cloud node image (apps/site/api/container) proved raea renders in a Linux container
  on Mesa's llvmpipe. sdk/cloud makes them one module a developer deploys on their own account, wherever they run
  containers. raea sells no cloud compute: the server, the GPU and the agent's subscription are the developer's.

  Background:
    Given one image, raea-cloud: Node 24 on Debian trixie, raea's Linux release (x86_64, checked against its .sha256),
      the server (sdk/cloud/server) and the agents named when it was built (AGENTS, ACP registry ids)
    And one secret, RAEA_CLOUD_TOKEN, that every request but /healthz carries

  Rule: the image holds raea and any ACP agent
    Scenario: CL1. The image is built from a raea release
      Then `docker build --build-arg RAEA_VERSION=V sdk/cloud` downloads raea V's Linux tarball, checks it against
        the release's .sha256 (and RAEA_SHA256 when given), and `raea --version` in the image says V
      And it runs as a user named raea, the workspace ~/work in raea's home folder
    Scenario: CL2. Agents come from the ACP registry by id
      Given AGENTS="kimi-code claude-acp codex-acp gemini opencode" (any ids the registry lists)
      Then each is installed from its registry entry when the image is built: an npx entry with npm at the registry's
        version, a binary entry from its linux archive checked by sha256; a uvx entry is refused with a reason
      And /opt/raea-cloud/agents/ID/spawn.json says how to start it, and the registry's entry is kept beside it
      And RAEA_AGENT picks the default agent; RAEA_AGENT_CMD runs any other ACP agent by its command line
    Scenario: CL3. Each agent gets raea's skill, matched to the image's raea
      Then on every start `raea skills install --agent A` runs for each installed agent raea has skills for (kimi,
        claude, codex, gemini), so a mounted home from an older image is brought up to date

  Rule: one port, one token
    Scenario: CL4. The server answers on :8080
      Then GET /healthz is "ok" without a token, for platform health checks
      And GET /v1/health says raea's version, the default agent, the installed agents and the workspace
      And every other route refuses a request without the token (Authorization: Bearer, or ?token= for a browser's
        WebSocket) with 401, compared in constant time, and the server won't start without one
    Scenario: CL5. Files go in and out of the workspace
      Then PUT /v1/files?path=P writes P (its folders made), GET reads it or lists a folder as JSON
      And a path that leaves the workspace (.., an absolute path, a symlink out) is refused with 403
    Scenario: CL6. raea runs without an agent
      Then POST /v1/run {args, cwd} runs `raea ARGS` in the workspace, streaming {out}, {err} and last {exit} lines
      And the token and the state keys are never in its environment, and a dropped request stops the run
    Scenario: CL7. ACP over a WebSocket
      Then each WebSocket on /v1/acp?agent=ID is one agent process in the workspace: a text frame is one JSON-RPC
        message to its stdin, each line it prints is a frame back, and closing either end ends the other
      And the agent's environment is the container's (its API keys) without RAEA_CLOUD_TOKEN or the state keys
    Scenario: CL8. Agents that sign in by a device code sign in through the server
      Then POST /v1/login?agent=kimi-code starts Kimi Code's login and returns the URL to approve; the login is kept
        in the home folder
      And agents that take a key (claude-acp, codex-acp, gemini) take it from the environment

  Rule: it deploys anywhere containers run
    Scenario: CL9. Docker and Compose
      Then deploy/compose.yaml runs it with the home folder on a named volume, and deploy/compose.gpu.yaml adds an
        NVIDIA GPU (graphics capability, so Vulkan sees it)
    Scenario: CL10. The platforms
      Then deploy/ has a Fly Machines app (a volume, stopped when idle, woken by a request), Kubernetes manifests
        (a PersistentVolumeClaim, a Secret, a Deployment and a Service), and a Cloudflare Container (a Worker in
        front, the container asleep when idle)
      And where the disk doesn't outlive the container (Cloudflare), RAEA_CLOUD_STATE names an S3-compatible object
        (R2, S3) the home folder is restored from on start and saved to on stop
    Scenario: CL11. The TypeScript client
      Then `new RaeaCloud({ url, token })` from sdk/cloud/src has health, read, write, list, run, login and session;
        a session is ACP: prompt(text, onUpdate) resolves with the stop reason, permission requests go to
        onPermission (allowed once by default: the container is the sandbox)
      And it runs on Node 22+, Bun and in a browser (fetch and WebSocket only)

  Rule: it is proven end to end
    Scenario: CL12. A stand-in agent, then a real one
      Then test/smoke.sh builds the image, starts it, and checks CL4-CL7 with test/echo-agent.mjs (a scripted ACP
        agent): the token refused, a path escape refused, a frame rendered by /v1/run, an ACP turn round-tripped
      And with Kimi Code signed in, one real turn renders a title card with raea in the container, and the file
        comes back through /v1/files
