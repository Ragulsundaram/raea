// Web Audio (AudioBuffer, OfflineAudioContext.decodeAudioData) and WebSocket (the host's sockets).
import { call, on } from "../wire";
import { B64, Blob_, DOMEx, DOMException_, EventTarget_, TOKEN, URL_, b64bytes, baseHref, btoa_, bytesOf, event, g, latin1, named, put, tag, utf8 } from "./all";

// ---------------------------------------------------------------- Web Audio: AudioBuffer, OfflineAudioContext.decodeAudioData
// The part of Web Audio a page reads a file's sound through (a waveform's peaks): decodeAudioData, decoded by the host
// ("audio.decode", raea-runtime-js's webaudio.rs) and resampled to the context's rate as Chrome does, into an AudioBuffer of
// Float32Arrays. There's no graph yet: an OfflineAudioContext renders its length of silence, and AudioContext, whose
// nodes play, isn't installed, so a page that looks for it finds it missing rather than half there.

// base64 to bytes and back for the large payloads (a file's bytes out, its samples back), by table, not by string
const B64_AT = new Uint8Array(128);

for (let i = 0; i < B64.length; i++) B64_AT[B64.charCodeAt(i)] = i;

const b64decode = (s: string) => {
  const n = s.length - (s.endsWith("==") ? 2 : s.endsWith("=") ? 1 : 0);
  const out = new Uint8Array(Math.floor((n * 3) / 4));
  let o = 0;
  for (let i = 0; i < n; i += 4) {
    const a = B64_AT[s.charCodeAt(i)], b = B64_AT[s.charCodeAt(i + 1)], c = B64_AT[s.charCodeAt(i + 2)], d = B64_AT[s.charCodeAt(i + 3)];
    out[o++] = (a << 2) | (b >> 4);
    if (o < out.length) out[o++] = ((b & 15) << 4) | (c >> 2);
    if (o < out.length) out[o++] = ((c & 3) << 6) | d;
  }
  return out;
};

const b64encode = (x: Uint8Array) => btoa_(latin1(x));

class AudioBuffer_ {
  readonly sampleRate: number;
  readonly length: number;
  readonly numberOfChannels: number;
  private _data: Float32Array[];
  constructor(o: { numberOfChannels?: number; length: number; sampleRate: number }) {
    const n = o?.numberOfChannels ?? 1;
    if (!(n >= 1 && n <= 32)) throw new DOMException_(`The number of channels provided (${n}) is outside the range [1, 32].`, "NotSupportedError");
    if (!(o.length >= 1)) throw new DOMException_(`The number of frames provided (${o.length}) is less than or equal to the minimum bound (0).`, "NotSupportedError");
    if (!(o.sampleRate >= 3000 && o.sampleRate <= 768000)) throw new DOMException_(`The sample rate provided (${o.sampleRate}) is outside the range [3000, 768000].`, "NotSupportedError");
    this.sampleRate = o.sampleRate;
    this.length = Math.floor(o.length);
    this.numberOfChannels = n;
    this._data = Array.from({ length: n }, () => new Float32Array(this.length));
  }
  get duration() {
    return this.length / this.sampleRate;
  }
  getChannelData(c: number) {
    if (!(c >= 0 && c < this.numberOfChannels)) throw new DOMException_(`The channel index provided (${c}) is outside the range [0, ${this.numberOfChannels}).`, "IndexSizeError");
    return this._data[c];
  }
  copyFromChannel(dest: Float32Array, c: number, start = 0) {
    const d = this.getChannelData(c);
    dest.set(d.subarray(start, start + dest.length));
  }
  copyToChannel(src: Float32Array, c: number, start = 0) {
    const d = this.getChannelData(c);
    d.set(src.subarray(0, Math.max(0, d.length - start)), start);
  }
}

tag(AudioBuffer_, "AudioBuffer");

class OfflineAudioContext_ extends EventTarget_ {
  readonly sampleRate: number;
  readonly length: number;
  private _channels: number;
  state = "suspended";
  currentTime = 0;
  constructor(a: number | { numberOfChannels?: number; length: number; sampleRate: number }, length?: number, sampleRate?: number) {
    super();
    const o = typeof a === "object" ? a : { numberOfChannels: a, length: length!, sampleRate: sampleRate! };
    // the same bounds as a buffer's (Chrome checks them here)
    new AudioBuffer_(o);
    this._channels = o.numberOfChannels ?? 1;
    this.length = Math.floor(o.length);
    this.sampleRate = o.sampleRate;
  }
  decodeAudioData(data: ArrayBuffer, ok?: (b: AudioBuffer_) => void, fail?: (e: unknown) => void): Promise<AudioBuffer_> {
    const p = new Promise<AudioBuffer_>((resolve, reject) => {
      if (!(data instanceof ArrayBuffer)) return reject(new TypeError("Failed to execute 'decodeAudioData' on 'BaseAudioContext': parameter 1 is not of type 'ArrayBuffer'."));
      if (data.byteLength === 0) return reject(new DOMException_("Unable to decode audio data", "EncodingError"));
      const bytes = new Uint8Array(data);
      call<{ rate: number; length: number; channels: string[] }>("audio.decode", { bytes: b64encode(bytes), rate: this.sampleRate }).then(
        (a) => {
          const b = new AudioBuffer_({ numberOfChannels: a.channels.length, length: Math.max(1, a.length), sampleRate: a.rate });
          a.channels.forEach((c, i) => b.getChannelData(i).set(new Float32Array(b64decode(c).buffer)));
          resolve(b);
        },
        () => reject(new DOMException_("Unable to decode audio data", "EncodingError")),
      );
    });
    p.then(ok, fail);
    return p;
  }
  startRendering(): Promise<AudioBuffer_> {
    this.state = "running";
    const b = new AudioBuffer_({ numberOfChannels: this._channels, length: this.length, sampleRate: this.sampleRate });
    return new Promise((ok) =>
      g.setTimeout(() => {
        this.state = "closed";
        ok(b);
      }, 0),
    );
  }
}

tag(OfflineAudioContext_, "OfflineAudioContext");

put("AudioBuffer", named(AudioBuffer_, "AudioBuffer"));

put("OfflineAudioContext", named(OfflineAudioContext_, "OfflineAudioContext"));

// ---------------------------------------------------------------- WebSocket (the host's sockets, "net.ws.*")

const sockets = new Map<number, WebSocket_>();

let socketSeq = 1;

let wsHeard = false;

class WebSocket_ extends EventTarget_ {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;
  readonly url: string;
  readyState = 0;
  protocol = "";
  extensions = "";
  bufferedAmount = 0;
  binaryType: "blob" | "arraybuffer" = "blob";
  onopen: any = null;
  onmessage: any = null;
  onerror: any = null;
  onclose: any = null;
  _id: number;
  constructor(url: unknown, protocols?: string | string[]) {
    super();
    let u: URL_;
    try {
      u = new URL_(String(url), baseHref());
    } catch {
      throw new (DOMEx())(`Failed to construct 'WebSocket': The URL '${url}' is invalid.`, "SyntaxError");
    }
    if (u.protocol === "http:") u.protocol = "ws:";
    if (u.protocol === "https:") u.protocol = "wss:";
    if (u.protocol !== "ws:" && u.protocol !== "wss:") throw new (DOMEx())(`Failed to construct 'WebSocket': The URL's scheme must be either 'http', 'https', 'ws', or 'wss'. '${u.protocol}' is not allowed.`, "SyntaxError");
    if (u.hash) throw new (DOMEx())(`Failed to construct 'WebSocket': The URL contains a fragment identifier ('${u.hash}'). Fragment identifiers are not allowed in WebSocket URLs.`, "SyntaxError");
    this.url = u.href;
    const ps = protocols === undefined ? [] : Array.isArray(protocols) ? protocols.map(String) : [String(protocols)];
    if (new Set(ps).size !== ps.length || ps.some((p) => !TOKEN.test(p))) throw new (DOMEx())(`Failed to construct 'WebSocket': The subprotocol '${ps.find((p, i) => !TOKEN.test(p) || ps.indexOf(p) !== i)}' is invalid.`, "SyntaxError");
    this._id = socketSeq++;
    sockets.set(this._id, this);
    if (!wsHeard) {
      wsHeard = true;
      on<any>("net.ws", (m) => sockets.get(m.id)?._hear(m));
    }
    call("net.ws.open", { id: this._id, url: this.url, protocols: ps }).catch(() => this._hear({ id: this._id, ev: "close", code: 1006, reason: "", clean: false, error: true }));
  }
  _hear(m: { ev: string; data?: string; binary?: boolean; code?: number; reason?: string; clean?: boolean; protocol?: string; error?: boolean }) {
    if (this.readyState === 3) return;
    if (m.ev === "open") {
      this.readyState = 1;
      this.protocol = m.protocol ?? "";
      this.dispatchEvent(event("open"));
    } else if (m.ev === "message") {
      if (this.readyState !== 1) return;
      let data: unknown = m.data ?? "";
      if (m.binary) {
        const b = b64bytes(String(data));
        data = this.binaryType === "arraybuffer" ? b.buffer : new Blob_([b]);
      }
      this.dispatchEvent(event("message", { data, origin: new URL_(this.url).origin, lastEventId: "", source: null, ports: [] }));
    } else if (m.ev === "close") {
      const failed = m.error || (!m.clean && this.readyState === 0);
      this.readyState = 3;
      sockets.delete(this._id);
      if (failed) this.dispatchEvent(event("error"));
      this.dispatchEvent(event("close", { code: m.code ?? 1006, reason: m.reason ?? "", wasClean: !!m.clean }));
    }
  }
  send(data: unknown) {
    if (this.readyState === 0) throw new (DOMEx())("Failed to execute 'send' on 'WebSocket': Still in CONNECTING state.", "InvalidStateError");
    if (this.readyState !== 1) return;
    if (typeof data === "string") call("net.ws.send", { id: this._id, data }).catch(() => {});
    else {
      const b = data instanceof Blob_ ? data._b : bytesOf(data);
      call("net.ws.send", { id: this._id, data: b64encode(b), binary: true }).catch(() => {});
    }
  }
  close(code?: number, reason = "") {
    if (code !== undefined && code !== 1000 && (code < 3000 || code > 4999)) throw new (DOMEx())(`Failed to execute 'close' on 'WebSocket': The close code must be either 1000, or between 3000 and 4999. ${code} is neither.`, "InvalidAccessError");
    if (utf8(String(reason)).length > 123) throw new (DOMEx())("Failed to execute 'close' on 'WebSocket': The close reason must not be greater than 123 UTF-8 bytes.", "SyntaxError");
    if (this.readyState >= 2) return;
    if (this.readyState === 0) {
      this.readyState = 2;
      call("net.ws.close", { id: this._id, code: code ?? 1000, reason }).catch(() => {});
      queueMicrotask(() => this._hear({ id: this._id, ev: "close", code: 1006, reason: "", clean: false, error: true }));
      return;
    }
    this.readyState = 2;
    call("net.ws.close", { id: this._id, code: code ?? 1000, reason: String(reason) }).catch(() => this._hear({ id: this._id, ev: "close", code: code ?? 1005, reason, clean: true }));
  }
}

put("WebSocket", named(WebSocket_, "WebSocket"));
