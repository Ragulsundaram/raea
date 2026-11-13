// #124: bytes become a binary string a slice at a time, so a blob of a few MB doesn't overflow the stack the way
// `String.fromCharCode(...bytes)` does (it passes every byte as an argument; the Windows app crashed at launch).
import { expect, test } from "bun:test";
import { btoa_, latin1 } from "../src/platform/all";

const bytes = (n: number) => {
  const b = new Uint8Array(n);
  for (let i = 0; i < n; i++) b[i] = (i * 31 + (i >> 9)) & 255;
  return b;
};

test("a 4 MB buffer is one char per byte, where the spread overflows", () => {
  const b = bytes(4 << 20);
  expect(() => String.fromCharCode(...b)).toThrow(RangeError);
  const s = latin1(b);
  expect(s.length).toBe(b.length);
  for (const i of [0, 1, 0x7fff, 0x8000, 0x8001, b.length - 1]) expect(s.charCodeAt(i)).toBe(b[i]);
});

test("its base64 is the bytes' base64, across a slice boundary", () => {
  const b = bytes(0x8000 + 5);
  expect(btoa_(latin1(b))).toBe(Buffer.from(b).toString("base64"));
  expect(latin1(new Uint8Array(0))).toBe("");
});
