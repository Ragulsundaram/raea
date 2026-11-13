// crypto (not cryptographic), performance, Blob, File, FileReader, FormData and ReadableStream.
import { DOMEx, EventTarget_, TextDecoder_, at, btoa_, bytesOf, entries, event, fromUtf8, g, latin1, named, put, tag, utf8 } from "./all";

// ---------------------------------------------------------------- crypto (not cryptographic: no secure source here)

function rand(a: ArrayBufferView) {
  const b = new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
  if (b.length > 65536) throw new (DOMEx())(`Failed to execute 'getRandomValues' on 'Crypto': The ArrayBufferView's byte length (${b.length}) exceeds the number of bytes of entropy available via this API (65536).`, "QuotaExceededError");
  for (let i = 0; i < b.length; i++) b[i] = (Math.random() * 256) | 0;
  return a;
}

export function uuid(_n?: number) {
  const b = rand(new Uint8Array(16)) as Uint8Array;
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  const h = [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

put("crypto", { getRandomValues: rand, randomUUID: () => uuid() });

// ---------------------------------------------------------------- performance: marks, measures, timeOrigin

if (g.performance) {
  const p = g.performance;
  const entries: any[] = [];
  p.timeOrigin ??= Date.now() - (p.now?.() ?? 0);
  p.mark ??= (name: string, o: { startTime?: number; detail?: unknown } = {}) => {
    const e = { name: String(name), entryType: "mark", startTime: o.startTime ?? p.now(), duration: 0, detail: o.detail ?? null, toJSON() { return { ...this }; } };
    entries.push(e);
    return e;
  };
  p.measure ??= (name: string, a?: string | { start?: number | string; end?: number | string; duration?: number; detail?: unknown }, b?: string) => {
    const at = (x: unknown, d: number) => (x === undefined ? d : typeof x === "number" ? x : (entries.filter((e) => e.name === x && e.entryType === "mark").pop()?.startTime ?? (() => { throw new (DOMEx())(`Failed to execute 'measure' on 'Performance': The mark '${x}' does not exist.`, "SyntaxError"); })()));
    let start = 0;
    let end = p.now();
    let detail = null;
    if (typeof a === "object" && a) {
      start = at(a.start, a.duration !== undefined && a.end !== undefined ? at(a.end, 0) - a.duration : 0);
      end = a.end !== undefined ? at(a.end, end) : a.duration !== undefined ? start + a.duration : end;
      detail = a.detail ?? null;
    } else {
      start = at(a, 0);
      end = at(b, end);
    }
    const e = { name: String(name), entryType: "measure", startTime: start, duration: end - start, detail, toJSON() { return { ...this }; } };
    entries.push(e);
    return e;
  };
  p.getEntries ??= () => entries.slice();
  p.getEntriesByType ??= (t: string) => entries.filter((e) => e.entryType === t);
  p.getEntriesByName ??= (n: string, t?: string) => entries.filter((e) => e.name === n && (!t || e.entryType === t));
  p.clearMarks ??= (n?: string) => void entries.splice(0, entries.length, ...entries.filter((e) => e.entryType !== "mark" || (n !== undefined && e.name !== n)));
  p.clearMeasures ??= (n?: string) => void entries.splice(0, entries.length, ...entries.filter((e) => e.entryType !== "measure" || (n !== undefined && e.name !== n)));
  p.clearResourceTimings ??= () => {};
  p.setResourceTimingBufferSize ??= () => {};
  p.toJSON ??= () => ({ timeOrigin: p.timeOrigin });
}

// ---------------------------------------------------------------- Blob, File, FileReader

type Part = Uint8Array;

export class Blob_ {
  _b: Uint8Array;
  readonly type: string;
  constructor(parts: unknown[] = [], o: { type?: string; endings?: string } = {}) {
    const bs: Part[] = [];
    for (const p of parts ?? []) {
      if (p instanceof Blob_) bs.push(p._b);
      else if (ArrayBuffer.isView(p) || p instanceof ArrayBuffer) bs.push(bytesOf(p).slice());
      else bs.push(utf8(o.endings === "native" ? String(p).replace(/\r?\n/g, "\n") : String(p)));
    }
    let n = 0;
    for (const b of bs) n += b.length;
    this._b = new Uint8Array(n);
    let at = 0;
    for (const b of bs) {
      this._b.set(b, at);
      at += b.length;
    }
    const t = String(o.type ?? "");
    this.type = /^[\x20-\x7e]*$/.test(t) ? t.toLowerCase() : "";
  }
  get size() {
    return this._b.length;
  }
  slice(start = 0, end = this.size, type = "") {
    const n = this.size;
    const s = start < 0 ? Math.max(n + start, 0) : Math.min(start, n);
    const e = end < 0 ? Math.max(n + end, 0) : Math.min(end, n);
    const b = new Blob_([], { type });
    b._b = this._b.slice(s, Math.max(s, e));
    return b;
  }
  text() {
    return Promise.resolve(fromUtf8(this._b, false, false)[0]);
  }
  arrayBuffer() {
    return Promise.resolve(this._b.slice().buffer);
  }
  bytes() {
    return Promise.resolve(this._b.slice());
  }
  stream() {
    return streamOf(this._b);
  }
}

export class File_ extends Blob_ {
  readonly name: string;
  readonly lastModified: number;
  readonly webkitRelativePath = "";
  constructor(parts: unknown[], name: string, o: { type?: string; lastModified?: number } = {}) {
    if (arguments.length < 2) throw new TypeError(`Failed to construct 'File': 2 arguments required, but only ${arguments.length} present.`);
    super(parts, o);
    this.name = String(name);
    this.lastModified = o.lastModified ?? Date.now();
  }
}

put("Blob", named(Blob_, "Blob"));

put("File", named(File_, "File"));

tag(Blob_, "Blob");

tag(File_, "File");

class FileReader_ extends EventTarget_ {
  static EMPTY = 0;
  static LOADING = 1;
  static DONE = 2;
  readyState = 0;
  result: unknown = null;
  error: unknown = null;
  onload: any = null;
  onloadstart: any = null;
  onloadend: any = null;
  onprogress: any = null;
  onerror: any = null;
  onabort: any = null;
  _aborted = false;
  _read(b: Blob_, as: (x: Uint8Array) => unknown) {
    if (this.readyState === 1) throw new (DOMEx())("Failed to execute 'read' on 'FileReader': The object is already busy reading Blobs.", "InvalidStateError");
    this.readyState = 1;
    this.result = null;
    this._aborted = false;
    queueMicrotask(() => this.dispatchEvent(event("loadstart", { loaded: 0, total: b.size, lengthComputable: true })));
    g.setTimeout(() => {
      if (this._aborted) return;
      this.result = as(b._b);
      this.readyState = 2;
      this.dispatchEvent(event("progress", { loaded: b.size, total: b.size, lengthComputable: true }));
      this.dispatchEvent(event("load", { loaded: b.size, total: b.size, lengthComputable: true }));
      this.dispatchEvent(event("loadend", { loaded: b.size, total: b.size, lengthComputable: true }));
    }, 0);
  }
  readAsText(b: Blob_, enc = "utf-8") {
    this._read(b, (x) => new TextDecoder_(enc).decode(x));
  }
  readAsArrayBuffer(b: Blob_) {
    this._read(b, (x) => x.slice().buffer);
  }
  readAsBinaryString(b: Blob_) {
    this._read(b, (x) => latin1(x));
  }
  readAsDataURL(b: Blob_) {
    this._read(b, (x) => `data:${b.type || "application/octet-stream"};base64,${btoa_(latin1(x))}`);
  }
  abort() {
    if (this.readyState !== 1) return;
    this._aborted = true;
    this.readyState = 2;
    this.result = null;
    this.dispatchEvent(event("abort"));
    this.dispatchEvent(event("loadend"));
  }
}

put("FileReader", named(FileReader_, "FileReader"));

// ---------------------------------------------------------------- FormData

export class FormData_ {
  _l: [string, string | File_][] = [];
  constructor(form?: any, submitter?: any) {
    if (form === undefined || form === null) return;
    if (typeof form !== "object" || !form.elements) throw new TypeError("Failed to construct 'FormData': parameter 1 is not of type 'HTMLFormElement'.");
    for (const el of [...form.elements]) {
      const name = el.getAttribute?.("name") ?? el.name;
      const type = String(el.type ?? el.getAttribute?.("type") ?? "").toLowerCase();
      const tagn = String(el.localName ?? el.tagName ?? "").toLowerCase();
      if (!name || el.disabled || el.hasAttribute?.("disabled") || el.closest?.("fieldset[disabled]")) continue;
      if (tagn === "button" || ["submit", "button", "reset", "image"].includes(type)) {
        if (el === submitter) this._l.push([name, String(el.value ?? el.getAttribute?.("value") ?? "")]);
        continue;
      }
      if ((type === "checkbox" || type === "radio") && !el.checked) continue;
      // a checkbox's value: its attribute, or the value React set as a property (the DOM layer keeps it apart)
      if (type === "checkbox" || type === "radio") this._l.push([name, String(el.getAttribute?.("value") ?? el._value ?? "on")]);
      else if (type === "file") for (const f of el.files ?? []) this._l.push([name, f]);
      else if (tagn === "select") {
        const chosen = el.selectedOptions ? [...el.selectedOptions] : [...(el.options ?? [])].filter((o: any) => o.selected);
        if (chosen.length) for (const o of chosen) this._l.push([name, String(o.value)]);
        else if (!el.multiple && el.value !== undefined && el.value !== "") this._l.push([name, String(el.value)]);
      } else this._l.push([name, String(el.value ?? "")]);
    }
  }
  _v(v: unknown, f?: string): string | File_ {
    if (v instanceof Blob_) return v instanceof File_ && f === undefined ? v : new File_([v], f ?? (v instanceof File_ ? v.name : "blob"), { type: v.type });
    return String(v);
  }
  append(k: string, v: unknown, f?: string) {
    this._l.push([String(k), this._v(v, f)]);
  }
  set(k: string, v: unknown, f?: string) {
    k = String(k);
    const i = this._l.findIndex(([a]) => a === k);
    const e: [string, string | File_] = [k, this._v(v, f)];
    if (i < 0) this._l.push(e);
    else this._l = this._l.filter(([a], j) => a !== k || j === i).map((x) => (x[0] === k ? e : x));
  }
  get(k: string) {
    return this._l.find(([a]) => a === String(k))?.[1] ?? null;
  }
  getAll(k: string) {
    return this._l.filter(([a]) => a === String(k)).map(([, b]) => b);
  }
  has(k: string) {
    return this._l.some(([a]) => a === String(k));
  }
  delete(k: string) {
    this._l = this._l.filter(([a]) => a !== String(k));
  }
  forEach(fn: (v: unknown, k: string, f: FormData_) => void, self?: unknown) {
    for (const [k, v] of this._l.slice()) fn.call(self, v, k, this);
  }
  keys() {
    return this._l.map(([k]) => k)[Symbol.iterator]();
  }
  values() {
    return this._l.map(([, v]) => v)[Symbol.iterator]();
  }
  entries() {
    return this._l.map(([k, v]) => [k, v])[Symbol.iterator]();
  }
  [Symbol.iterator]() {
    return this.entries();
  }
}

put("FormData", named(FormData_, "FormData"));

tag(FormData_, "FormData");

// ---------------------------------------------------------------- ReadableStream (what a body needs: a reader)

export function streamOf(bytes: Uint8Array) {
  let done = false;
  return new g.ReadableStream({
    pull(c: any) {
      if (!done && bytes.length) c.enqueue(bytes);
      done = true;
      c.close();
    },
  });
}

class ReadableStream_ {
  _q: unknown[] = [];
  _closed = false;
  _err: unknown = undefined;
  _waiting: { ok: (r: unknown) => void; err: (e: unknown) => void }[] = [];
  _src: any;
  _pulling = false;
  locked = false;
  constructor(src: any = {}) {
    this._src = src;
    const c = {
      enqueue: (v: unknown) => {
        const w = this._waiting.shift();
        if (w) w.ok({ value: v, done: false });
        else this._q.push(v);
      },
      close: () => {
        this._closed = true;
        for (const w of this._waiting.splice(0)) w.ok({ value: undefined, done: true });
      },
      error: (e: unknown) => {
        this._err = e;
        for (const w of this._waiting.splice(0)) w.err(e);
      },
      get desiredSize() {
        return 1;
      },
    };
    this._c = c;
    try {
      src.start?.(c);
    } catch (e) {
      c.error(e);
    }
  }
  _c: any;
  _pull() {
    if (this._pulling || this._closed || !this._src.pull) return;
    this._pulling = true;
    Promise.resolve(this._src.pull(this._c)).finally(() => (this._pulling = false));
  }
  getReader() {
    if (this.locked) throw new TypeError("Failed to execute 'getReader' on 'ReadableStream': ReadableStreamDefaultReader constructor can only accept readable streams that are not yet locked to a reader");
    this.locked = true;
    const s = this;
    return {
      read(): Promise<{ value: unknown; done: boolean }> {
        if (s._err !== undefined) return Promise.reject(s._err);
        if (s._q.length) return Promise.resolve({ value: s._q.shift(), done: false });
        if (s._closed) return Promise.resolve({ value: undefined, done: true });
        const p = new Promise<any>((ok, err) => s._waiting.push({ ok, err }));
        s._pull();
        return p;
      },
      releaseLock() {
        s.locked = false;
      },
      cancel(r?: unknown) {
        s._closed = true;
        s._q = [];
        return Promise.resolve(s._src.cancel?.(r));
      },
      get closed() {
        return Promise.resolve();
      },
    };
  }
  cancel(r?: unknown) {
    this._closed = true;
    return Promise.resolve(this._src.cancel?.(r));
  }
  async *[Symbol.asyncIterator]() {
    const r = this.getReader();
    for (;;) {
      const { value, done } = await r.read();
      if (done) return;
      yield value;
    }
  }
}

put("ReadableStream", named(ReadableStream_, "ReadableStream"));
