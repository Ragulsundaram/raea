// Headers, Request, Response and fetch.
import { call } from "../wire";
import { AbortSignal_, Blob_, File_, FormData_, URLSearchParams_, URL_, b64bytes, blobs, btoa_, bytesOf, fromUtf8, g, latin1, named, put, streamOf, tag, utf8, uuid } from "./all";

// ---------------------------------------------------------------- Headers, Request, Response, fetch

export const TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

class Headers_ {
  _m = new Map<string, string[]>();
  constructor(init?: unknown) {
    if (init instanceof Headers_) init.forEach((v, k) => this.append(k, v));
    else if (Array.isArray(init) || (init && typeof (init as any)[Symbol.iterator] === "function")) {
      for (const p of init as Iterable<unknown>) {
        const a = [...(p as Iterable<unknown>)];
        if (a.length !== 2) throw new TypeError("Failed to construct 'Headers': Invalid value");
        this.append(String(a[0]), String(a[1]));
      }
    } else if (init && typeof init === "object") for (const k of Object.keys(init)) this.append(k, String((init as any)[k]));
  }
  _n(k: string) {
    k = String(k);
    if (!TOKEN.test(k)) throw new TypeError(`Failed to execute 'append' on 'Headers': Invalid name`);
    return k.toLowerCase();
  }
  append(k: string, v: string) {
    const n = this._n(k);
    const l = this._m.get(n) ?? [];
    l.push(String(v).trim());
    this._m.set(n, l);
  }
  set(k: string, v: string) {
    this._m.set(this._n(k), [String(v).trim()]);
  }
  get(k: string) {
    const l = this._m.get(this._n(k));
    return l ? (this._n(k) === "set-cookie" ? l.join(", ") : l.join(", ")) : null;
  }
  getSetCookie() {
    return (this._m.get("set-cookie") ?? []).slice();
  }
  has(k: string) {
    return this._m.has(this._n(k));
  }
  delete(k: string) {
    this._m.delete(this._n(k));
  }
  forEach(fn: (v: string, k: string, h: Headers_) => void, self?: unknown) {
    for (const [k, v] of this.entries()) fn.call(self, v, k, this);
  }
  *entries(): IterableIterator<[string, string]> {
    for (const k of [...this._m.keys()].sort()) {
      if (k === "set-cookie") for (const v of this._m.get(k)!) yield [k, v];
      else yield [k, this._m.get(k)!.join(", ")];
    }
  }
  *keys() {
    for (const [k] of this.entries()) yield k;
  }
  *values() {
    for (const [, v] of this.entries()) yield v;
  }
  [Symbol.iterator]() {
    return this.entries();
  }
}

put("Headers", named(Headers_, "Headers"));

tag(Headers_, "Headers");

/** A body's bytes and the content type it implies (fetch's "extract a body") */
function extract(body: unknown): [Uint8Array | null, string | null] {
  if (body === null || body === undefined) return [null, null];
  if (typeof body === "string") return [utf8(body), "text/plain;charset=UTF-8"];
  if (body instanceof Blob_) return [body._b, body.type || null];
  if (body instanceof URLSearchParams_) return [utf8(body.toString()), "application/x-www-form-urlencoded;charset=UTF-8"];
  if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) return [bytesOf(body).slice(), null];
  if (body instanceof FormData_) {
    const boundary = "----formdata-raea-" + uuid().replace(/-/g, "").slice(0, 16);
    const parts: unknown[] = [];
    for (const [k, v] of body._l) {
      const esc = (s: string) => s.replace(/\n/g, "%0A").replace(/\r/g, "%0D").replace(/"/g, "%22");
      if (typeof v === "string") parts.push(`--${boundary}\r\nContent-Disposition: form-data; name="${esc(k)}"\r\n\r\n${v.replace(/\r?\n/g, "\r\n")}\r\n`);
      else parts.push(`--${boundary}\r\nContent-Disposition: form-data; name="${esc(k)}"; filename="${esc(v.name)}"\r\nContent-Type: ${v.type || "application/octet-stream"}\r\n\r\n`, v, "\r\n");
    }
    parts.push(`--${boundary}--\r\n`);
    return [new Blob_(parts)._b, `multipart/form-data; boundary=${boundary}`];
  }
  return [utf8(String(body)), "text/plain;charset=UTF-8"];
}

class Body {
  _body: Uint8Array | null = null;
  _used = false;
  headers!: Headers_;
  _stream: any = null;
  get body() {
    if (this._body === null) return null;
    return (this._stream ??= streamOf(this._body));
  }
  get bodyUsed() {
    return this._used;
  }
  _take(): Promise<Uint8Array> {
    if (this._used) return Promise.reject(new TypeError("Failed to execute 'text' on 'Response': body stream already read"));
    this._used = true;
    return Promise.resolve(this._body ?? new Uint8Array(0));
  }
  text() {
    return this._take().then((b) => fromUtf8(b, false, false)[0]);
  }
  json() {
    return this.text().then((t) => JSON.parse(t));
  }
  arrayBuffer() {
    return this._take().then((b) => b.slice().buffer);
  }
  bytes() {
    return this._take().then((b) => b.slice());
  }
  blob() {
    return this._take().then((b) => new Blob_([b], { type: this.headers.get("content-type") ?? "" }));
  }
  formData() {
    return this.text().then((t) => {
      const ct = this.headers.get("content-type") ?? "";
      const fd = new FormData_();
      if (/application\/x-www-form-urlencoded/i.test(ct)) for (const [k, v] of new URLSearchParams_(t)) fd.append(k, v);
      else {
        const m = /boundary=([^;]+)/i.exec(ct);
        if (!m) throw new TypeError("Failed to fetch");
        for (const part of t.split("--" + m[1]).slice(1, -1)) {
          const [head, ...rest] = part.replace(/^\r\n/, "").split("\r\n\r\n");
          const name = /name="([^"]*)"/.exec(head)?.[1] ?? "";
          const file = /filename="([^"]*)"/.exec(head)?.[1];
          const val = rest.join("\r\n\r\n").replace(/\r\n$/, "");
          fd.append(name, file === undefined ? val : new File_([val], file, { type: /Content-Type: (.*)/i.exec(head)?.[1] ?? "" }));
        }
      }
      return fd;
    });
  }
}

export function origin() {
  try {
    return new URL_(g.location?.href ?? "http://localhost/").origin;
  } catch {
    return "null";
  }
}

export function baseHref() {
  return g.location?.href ?? "http://localhost/";
}

class Request_ extends Body {
  readonly url: string;
  readonly method: string;
  readonly headers: Headers_;
  readonly signal: AbortSignal_;
  readonly mode: string;
  readonly credentials: string;
  readonly cache: string;
  readonly redirect: string;
  readonly referrer = "about:client";
  readonly referrerPolicy = "";
  readonly integrity = "";
  readonly keepalive: boolean;
  readonly destination = "";
  readonly duplex = "half";
  constructor(input: unknown, init: any = {}) {
    super();
    const from = input instanceof Request_ ? input : null;
    let url: string;
    if (from) url = from.url;
    else {
      try {
        url = new URL_(String(input), baseHref()).href;
      } catch {
        throw new TypeError(`Failed to construct 'Request': Failed to parse URL from ${input}`);
      }
    }
    if (/^[^:]+:\/\/[^/]*@/.test(url) && new URL_(url).username) throw new TypeError(`Failed to construct 'Request': Request cannot be constructed from a URL that includes credentials: ${url}`);
    this.url = url;
    let method = String(init.method ?? from?.method ?? "GET");
    if (/^(get|head|post|put|delete|options|patch)$/i.test(method) && !/^patch$/i.test(method)) method = method.toUpperCase();
    if (!TOKEN.test(method)) throw new TypeError(`Failed to construct 'Request': '${method}' is not a valid HTTP method.`);
    if (/^(CONNECT|TRACE|TRACK)$/i.test(method)) throw new TypeError(`Failed to construct 'Request': '${method}' HTTP method is unsupported.`);
    this.method = method;
    this.headers = new Headers_(init.headers ?? from?.headers);
    this.signal = init.signal ?? from?.signal ?? new AbortSignal_();
    this.mode = init.mode ?? from?.mode ?? "cors";
    this.credentials = init.credentials ?? from?.credentials ?? "same-origin";
    this.cache = init.cache ?? from?.cache ?? "default";
    this.redirect = init.redirect ?? from?.redirect ?? "follow";
    this.keepalive = !!(init.keepalive ?? from?.keepalive);
    const body = init.body !== undefined ? init.body : null;
    if (body !== null && (method === "GET" || method === "HEAD")) throw new TypeError("Failed to construct 'Request': Request with GET/HEAD method cannot have body.");
    if (body !== null) {
      const [b, ct] = extract(body);
      this._body = b;
      if (ct && !this.headers.has("content-type")) this.headers.set("content-type", ct);
    } else if (from) {
      if (from._used) throw new TypeError("Failed to construct 'Request': Cannot construct a Request with a Request object that has already been used.");
      this._body = from._body;
    }
  }
  clone() {
    if (this._used) throw new TypeError("Failed to execute 'clone' on 'Request': Request body is already used");
    const r = new Request_(this);
    return r;
  }
}

const STATUS: Record<number, string> = { 200: "OK", 201: "Created", 204: "No Content", 301: "Moved Permanently", 302: "Found", 304: "Not Modified", 400: "Bad Request", 401: "Unauthorized", 403: "Forbidden", 404: "Not Found", 500: "Internal Server Error" };

class Response_ extends Body {
  readonly status: number;
  readonly statusText: string;
  readonly headers: Headers_;
  type = "default";
  url = "";
  redirected = false;
  constructor(body: unknown = null, init: { status?: number; statusText?: string; headers?: unknown } = {}) {
    super();
    const status = init.status ?? 200;
    if (status < 200 || status > 599) throw new RangeError(`Failed to construct 'Response': The status provided (${status}) is outside the range [200, 599].`);
    this.status = status;
    this.statusText = String(init.statusText ?? "");
    this.headers = new Headers_(init.headers);
    if (body !== null && body !== undefined) {
      if ([101, 103, 204, 205, 304].includes(status)) throw new TypeError("Failed to construct 'Response': Response with null body status cannot have body");
      const [b, ct] = extract(body);
      this._body = b;
      if (ct && !this.headers.has("content-type")) this.headers.set("content-type", ct);
    }
  }
  get ok() {
    return this.status >= 200 && this.status < 300;
  }
  clone() {
    if (this._used) throw new TypeError("Failed to execute 'clone' on 'Response': Response body is already used");
    const r = new Response_(null, { status: this.status, statusText: this.statusText, headers: this.headers });
    r._body = this._body;
    r.type = this.type;
    r.url = this.url;
    r.redirected = this.redirected;
    return r;
  }
  static json(data: unknown, init: { status?: number; statusText?: string; headers?: unknown } = {}) {
    const r = new Response_(JSON.stringify(data), init);
    r.headers.set("content-type", "application/json");
    return r;
  }
  static error() {
    const r = Object.create(Response_.prototype);
    Object.assign(r, { status: 0, statusText: "", headers: new Headers_(), type: "error", url: "", redirected: false, _body: null, _used: false });
    return r;
  }
  static redirect(url: string, status = 302) {
    if (![301, 302, 303, 307, 308].includes(status)) throw new RangeError("Failed to execute 'redirect' on 'Response': Invalid status code");
    const r = new Response_(null, { status, headers: { location: new URL_(url, baseHref()).href } });
    return r;
  }
}

put("Request", named(Request_, "Request"));

put("Response", named(Response_, "Response"));

tag(Request_, "Request");

tag(Response_, "Response");

function dataURL(url: string): Response_ {
  const m = /^data:([^,]*?)(;base64)?,(.*)$/is.exec(url);
  if (!m) throw new TypeError("Failed to fetch");
  const type = (m[1] || "text/plain;charset=US-ASCII").trim();
  let body: Uint8Array;
  if (m[2]) body = b64bytes(decodeURIComponent(m[3]));
  else {
    const s = m[3].replace(/%([0-9a-fA-F]{2})/g, (_x, h) => String.fromCharCode(parseInt(h, 16)));
    body = Uint8Array.from(s, (c) => c.charCodeAt(0) & 255);
  }
  const r = new Response_(body, { status: 200, headers: { "content-type": type } });
  (r as any).statusText = "OK";
  r.type = "basic";
  r.url = url;
  return r;
}

function fetch_(input: unknown, init?: any): Promise<Response_> {
  let req: Request_;
  try {
    req = new Request_(input, init);
  } catch (e) {
    return Promise.reject(e);
  }
  const signal = req.signal;
  if (signal.aborted) return Promise.reject(signal.reason);
  const abortable = <T>(p: Promise<T>) =>
    new Promise<T>((ok, err) => {
      signal.addEventListener("abort", () => err(signal.reason), { once: true });
      p.then(ok, err);
    });
  const u = new URL_(req.url);
  if (u.protocol === "data:") {
    if (req.method !== "GET") return Promise.reject(new TypeError("Failed to fetch"));
    // resolved after a task, as Chrome's is
    return abortable(new Promise((ok, err) => g.setTimeout(() => (signal.aborted ? err(signal.reason) : (() => { try { ok(dataURL(req.url)); } catch (e) { err(e); } })()), 0)));
  }
  if (u.protocol === "blob:") {
    const b = blobs.get(u.href.replace(/#.*$/, ""));
    return abortable(
      new Promise((ok, err) =>
        g.setTimeout(() => {
          if (!b || req.method !== "GET") return err(new TypeError("Failed to fetch"));
          const r = new Response_(b, { status: 200, headers: { "content-type": b.type, "content-length": String(b.size) } });
          (r as any).statusText = "OK";
          r.type = "basic";
          r.url = req.url;
          ok(r);
        }, 0),
      ),
    );
  }
  if (u.protocol !== "http:" && u.protocol !== "https:") return Promise.reject(new TypeError("Failed to fetch"));
  const headers: [string, string][] = [...req.headers];
  const body = req._body ? btoa_(latin1(req._body)) : null;
  return abortable(
    call<{ status: number; statusText?: string; headers: [string, string][]; body: string; url: string; redirected?: boolean }>("net.fetch", { url: req.url, method: req.method, headers, body, redirect: req.redirect }).then(
      (a) => {
        const r = new Response_(null, { status: a.status >= 200 ? a.status : 200, statusText: a.statusText ?? STATUS[a.status] ?? "", headers: a.headers });
        (r as any).status = a.status;
        r._body = [101, 103, 204, 205, 304].includes(a.status) || req.method === "HEAD" ? null : b64bytes(a.body ?? "");
        r.type = new URL_(req.url).origin === origin() ? "basic" : "cors";
        r.url = a.url ?? req.url;
        r.redirected = !!a.redirected;
        return r;
      },
      () => {
        throw new TypeError("Failed to fetch");
      },
    ),
  );
}

put("fetch", fetch_);
