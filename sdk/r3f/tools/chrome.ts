// Headless Chrome over the DevTools protocol, for tools/shoot.ts (a copy of sdk/playable/tools/chrome.ts, which isn't public): a page opened at a size,
// its console and any network request it tried collected, screenshots and touches, then Chrome stopped.
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

export interface Chrome {
  send(method: string, params?: object): Promise<any>;
  /** the page's console lines, in order */
  logs: string[];
  /** exceptions and network requests: either fails a check */
  problems: string[];
  shot(file: string): Promise<void>;
  touch(x: number, y: number, type: "touchStart" | "touchMove" | "touchEnd"): Promise<any>;
  /** an expression's value, read back */
  value(expression: string): Promise<unknown>;
  close(): void;
}

/** Chrome opened on `page` (a file or an http URL) at width x height, `scale` device pixels per CSS px */
export async function openChrome(page: string, width: number, height: number, opts: { scale?: number; mobile?: boolean; throttle?: number } = {}): Promise<Chrome> {
  const chrome = process.env.CHROME ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  const profile = mkdtempSync(resolve(tmpdir(), "raea-playable-chrome-"));
  const port = 9300 + Math.floor(Math.random() * 500);
  const proc = Bun.spawn([chrome, "--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, "--enable-unsafe-swiftshader", "--no-first-run", "about:blank"], { stdout: "ignore", stderr: "ignore" });

  let target: { webSocketDebuggerUrl: string } | undefined;
  for (let i = 0; i < 100 && !target; i++) {
    await Bun.sleep(100);
    try {
      const list = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as { type: string; webSocketDebuggerUrl: string }[];
      target = list.find((t) => t.type === "page");
    } catch {}
  }
  if (!target) throw new Error("chrome: DevTools never came up");

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const waiting = new Map<number, (v: any) => void>();
  const logs: string[] = [];
  const problems: string[] = [];
  ws.onmessage = (m) => {
    const msg = JSON.parse(String(m.data));
    if (msg.id && waiting.has(msg.id)) {
      waiting.get(msg.id)!(msg.result ?? msg.error);
      waiting.delete(msg.id);
    } else if (msg.method === "Runtime.consoleAPICalled") {
      logs.push(`${msg.params.type}: ${msg.params.args.map((a: any) => a.value ?? a.description).join(" ")}`);
    } else if (msg.method === "Runtime.exceptionThrown") {
      problems.push(`exception: ${msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text}`);
    } else if (msg.method === "Network.requestWillBeSent") {
      const url: string = msg.params.request.url;
      if (!/^(file|data|blob):|^http:\/\/127\.0\.0\.1/.test(url)) problems.push(`network request: ${url}`);
    }
  };
  const send = (method: string, params: object = {}) =>
    new Promise<any>((r) => {
      const n = ++id;
      waiting.set(n, r);
      ws.send(JSON.stringify({ id: n, method, params }));
    });

  await send("Runtime.enable");
  await send("Network.enable");
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: opts.scale ?? 2, mobile: opts.mobile ?? true });
  if (opts.mobile ?? true) await send("Emulation.setTouchEmulationEnabled", { enabled: true });
  if (opts.throttle) await send("Emulation.setCPUThrottlingRate", { rate: opts.throttle });
  await send("Page.enable");
  await send("Page.navigate", { url: /^https?:/.test(page) ? page : `file://${resolve(page.split("#")[0])}${page.includes("#") ? "#" + page.split("#")[1] : ""}` });

  return {
    send,
    logs,
    problems,
    async shot(file) {
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      await Bun.write(file, Buffer.from(data, "base64"));
    },
    touch: (x, y, type) => send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" ? [] : [{ x, y }] }),
    async value(expression) {
      const r = await send("Runtime.evaluate", { expression, returnByValue: true });
      return r.result?.value;
    },
    close() {
      ws.close();
      proc.kill("SIGKILL");
      rmSync(profile, { recursive: true, force: true });
    },
  };
}
