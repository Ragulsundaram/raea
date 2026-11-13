// TextEncoder, TextDecoder, atob and btoa.
import { DOMEx, named, put } from "./all";

// ---------------------------------------------------------------- TextEncoder, TextDecoder, atob, btoa

export function utf8(s: string): Uint8Array {
  const out: number[] = [];
  for (let i = 0; i < s.length; i++) {
    let c = s.charCodeAt(i);
    if (c >= 0xd800 && c <= 0xdbff && i + 1 < s.length) {
      const d = s.charCodeAt(i + 1);
      if (d >= 0xdc00 && d <= 0xdfff) {
        c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00);
        i++;
      } else c = 0xfffd;
    } else if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd;
    if (c < 0x80) out.push(c);
    else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
    else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
    else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
  }
  return new Uint8Array(out);
}

/** UTF-8 to a string, WHATWG's decoder: a bad sequence is one U+FFFD (or a throw when fatal); returns the bytes of an
 *  unfinished sequence at the end when `stream` keeps them for the next call. */
export function fromUtf8(b: Uint8Array, fatal: boolean, stream: boolean): [string, number] {
  let s = "";
  let i = 0;
  const bad = () => {
    if (fatal) throw new TypeError("The encoded data was not valid for encoding utf-8");
    s += "�";
  };
  while (i < b.length) {
    const c = b[i];
    if (c < 0x80) {
      s += String.fromCharCode(c);
      i++;
      continue;
    }
    let n = 0;
    let cp = 0;
    let lo = 0x80;
    let hi = 0xbf;
    if (c >= 0xc2 && c <= 0xdf) (n = 1), (cp = c & 0x1f);
    else if (c >= 0xe0 && c <= 0xef) {
      n = 2;
      cp = c & 0xf;
      if (c === 0xe0) lo = 0xa0;
      if (c === 0xed) hi = 0x9f;
    } else if (c >= 0xf0 && c <= 0xf4) {
      n = 3;
      cp = c & 7;
      if (c === 0xf0) lo = 0x90;
      if (c === 0xf4) hi = 0x8f;
    } else {
      bad();
      i++;
      continue;
    }
    let j = 1;
    for (; j <= n; j++) {
      if (i + j >= b.length) {
        if (stream) return [s, b.length - i];
        break;
      }
      const d = b[i + j];
      if (d < (j === 1 ? lo : 0x80) || d > (j === 1 ? hi : 0xbf)) break;
      cp = (cp << 6) | (d & 0x3f);
    }
    if (j <= n) {
      bad();
      i += j;
      continue;
    }
    s += String.fromCodePoint(cp);
    i += n + 1;
  }
  return [s, 0];
}

export const bytesOf = (x: any): Uint8Array =>
  x instanceof Uint8Array ? x : ArrayBuffer.isView(x) ? new Uint8Array(x.buffer, x.byteOffset, x.byteLength) : x instanceof ArrayBuffer ? new Uint8Array(x) : new Uint8Array(0);

class TextEncoder_ {
  get encoding() {
    return "utf-8";
  }
  encode(s = "") {
    return utf8(String(s));
  }
  encodeInto(s: string, dest: Uint8Array) {
    let read = 0;
    let written = 0;
    for (const ch of String(s)) {
      const b = utf8(ch);
      if (written + b.length > dest.length) break;
      dest.set(b, written);
      written += b.length;
      read += ch.length;
    }
    return { read, written };
  }
}

const LABELS: Record<string, string> = { "utf-8": "utf-8", utf8: "utf-8", "unicode-1-1-utf-8": "utf-8", "utf-16le": "utf-16le", "utf-16": "utf-16le", "iso-8859-1": "windows-1252", latin1: "windows-1252", "windows-1252": "windows-1252", ascii: "windows-1252", "us-ascii": "windows-1252" };

export class TextDecoder_ {
  readonly encoding: string;
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;
  _held = new Uint8Array(0);
  _bomSeen = false;
  constructor(label = "utf-8", o: { fatal?: boolean; ignoreBOM?: boolean } = {}) {
    const e = LABELS[String(label).trim().toLowerCase()];
    if (!e) throw new RangeError(`Failed to construct 'TextDecoder': The encoding label provided ('${label}') is invalid.`);
    this.encoding = e;
    this.fatal = !!o.fatal;
    this.ignoreBOM = !!o.ignoreBOM;
  }
  decode(input?: any, o: { stream?: boolean } = {}) {
    let b = input === undefined ? new Uint8Array(0) : bytesOf(input);
    if (this._held.length) {
      const m = new Uint8Array(this._held.length + b.length);
      m.set(this._held);
      m.set(b, this._held.length);
      b = m;
      this._held = new Uint8Array(0);
    }
    let s: string;
    if (this.encoding === "utf-16le") {
      const n = b.length - (b.length % 2);
      s = "";
      for (let i = 0; i < n; i += 2) s += String.fromCharCode(b[i] | (b[i + 1] << 8));
      if (n < b.length) {
        if (o.stream) this._held = b.slice(n);
        else s += "�";
      }
      if (!this.ignoreBOM && !this._bomSeen && s.charCodeAt(0) === 0xfeff) s = s.slice(1);
    } else if (this.encoding === "windows-1252") {
      s = "";
      for (const x of b) s += String.fromCharCode(x);
    } else {
      const [t, keep] = fromUtf8(b, this.fatal, !!o.stream);
      if (keep) this._held = b.slice(b.length - keep);
      s = t;
      if (!this.ignoreBOM && !this._bomSeen && s.charCodeAt(0) === 0xfeff) s = s.slice(1);
    }
    this._bomSeen = !!o.stream && (this._bomSeen || s.length > 0);
    return s;
  }
}

put("TextEncoder", named(TextEncoder_, "TextEncoder"));

put("TextDecoder", named(TextDecoder_, "TextDecoder"));

export const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/** Bytes as a binary string, one char per byte, 0x8000 bytes at a time: `String.fromCharCode(...bytes)` passes each
 *  byte as an argument, which overflows the stack on a blob of a few hundred KB (#124, Windows). */
export function latin1(b: Uint8Array): string {
  let s = "";
  for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000) as unknown as number[]);
  return s;
}

export function btoa_(s: string) {
  s = String(s);
  let out = "";
  for (let i = 0; i < s.length; i += 3) {
    const a = s.charCodeAt(i);
    const b = s.charCodeAt(i + 1);
    const c = s.charCodeAt(i + 2);
    for (const x of [a, b, c]) if (x > 255) throw new (DOMEx())("Failed to execute 'btoa' on 'Window': The string to be encoded contains characters outside of the Latin1 range.", "InvalidCharacterError");
    const n = (a << 16) | ((b || 0) << 8) | (c || 0);
    out += B64[n >> 18] + B64[(n >> 12) & 63] + (i + 1 < s.length ? B64[(n >> 6) & 63] : "=") + (i + 2 < s.length ? B64[n & 63] : "=");
  }
  return out;
}

function atob_(s: string) {
  s = String(s).replace(/[\t\n\f\r ]/g, "");
  if (s.length % 4 === 0) s = s.replace(/==?$/, "");
  if (s.length % 4 === 1 || /[^A-Za-z0-9+/]/.test(s)) throw new (DOMEx())("Failed to execute 'atob' on 'Window': The string to be decoded is not correctly encoded.", "InvalidCharacterError");
  let out = "";
  let bits = 0;
  let acc = 0;
  for (const ch of s) {
    acc = (acc << 6) | B64.indexOf(ch);
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out += String.fromCharCode((acc >> bits) & 255);
    }
  }
  return out;
}

put("btoa", btoa_);

put("atob", atob_);

export const b64bytes = (s: string) => Uint8Array.from(atob_(s), (c) => c.charCodeAt(0));
