// The composition's document and window (docs/layout-reads.feature LR7), over the tree bevy-react keeps (LR4) and the
// page's layout (LR1). document.body is the composition's root; querySelector and getElementById search it. An element
// made with createElement is a node of the page too: appended into the composition it's laid out, measured and drawn,
// and removed it's gone, which is how components measure a hidden copy. A canvas made there is an OffscreenCanvas
// (canvas2d.ts). document.fonts loads a family the composition asked for (loadFont) into the faces a canvas measures
// with. window is globalThis, with the composition's size, devicePixelRatio and getComputedStyle.
import { native } from "@raea/app/native";
import { OffscreenCanvas } from "./canvas2d";
import { querySelectorAll, type El, type Node } from "./dom-tree";
import { computed, declaration, parseCss, styleNow, writeStyle } from "./dom-style";
import { computedOf, viewportSize } from "./page-layout";
import { domRange } from "./dom-range";

type Bevy = { BevyElement: new (id: number, type: string, kind: string, props: Record<string, unknown>) => El; BevyTextNode: new (id: number, data: string) => Node & { data: string }; containerNode: (id: number) => El };
type Made = { made?: true; gone?: boolean };

const HOST = (globalThis as any).__bevyHost;
const send = (...ops: unknown[]) => HOST.op_flush(JSON.stringify(ops), false);
// ids of nodes the document makes: far above React's allocator (and below cancelRender's, 2^52)
let next = 2 ** 51;
const isText = (n: unknown): n is Node & { data: string } => typeof (n as { data?: unknown }).data === "string" && !Array.isArray((n as El).childNodes);

/** A node made here sent to the page again (one removed from it is despawned, its id forgotten), with its subtree. */
function create(n: Node & Made): void {
  if (isText(n)) {
    send({ op: "createText", id: n.nodeId, text: n.data });
  } else {
    const el = n as El;
    send({ op: "create", id: el.nodeId, kind: el.kind ?? el.type, props: { ...el.props, style: styleNow(el) } });
    for (const c of el.childNodes) {
      create(c as Node & Made);
      send({ op: "append", parent: el.nodeId, child: c.nodeId });
    }
  }
  n.gone = false;
}

function unlink(child: Node): void {
  const p = child.parentNode;
  if (!p) return;
  p.childNodes.splice(p.childNodes.indexOf(child), 1);
  (child as { parentNode: El | null }).parentNode = null;
}

/** Puts `child` under `parent`, before `before` (or last), on the page and in the tree. */
function insert(parent: El, child: Node & Made, before: Node | null): Node {
  if (child.parentNode) remove(child);
  if (child.gone) create(child);
  const at = before ? parent.childNodes.indexOf(before) : -1;
  if (at >= 0) send({ op: "insert", parent: parent.nodeId, child: child.nodeId, before: before!.nodeId });
  else send({ op: "append", parent: parent.nodeId, child: child.nodeId });
  parent.childNodes.splice(at >= 0 ? at : parent.childNodes.length, 0, child);
  (child as { parentNode: El | null }).parentNode = parent;
  return child;
}

function remove(child: Node & Made): Node {
  const p = child.parentNode;
  if (!p) return child;
  send({ op: "remove", parent: p.nodeId, child: child.nodeId });
  unlink(child);
  const mark = (n: Node & Made) => {
    n.gone = true;
    if (!isText(n)) (n as El).childNodes.forEach((c) => mark(c as Node & Made));
  };
  mark(child);
  return child;
}

/** Props from an attribute (class, for and style as React names them). */
const PROP: Record<string, string> = { class: "className", for: "htmlFor" };

function writers(bevy: Bevy): void {
  const p = bevy.BevyElement.prototype as Record<string, unknown>;
  const def = (name: string, f: (this: El, ...a: any[]) => unknown) => Object.defineProperty(p, name, { value: f, writable: true, configurable: true });
  def("appendChild", function (c: Node) {
    return insert(this, c, null);
  });
  def("append", function (...cs: (Node | string)[]) {
    for (const c of cs) insert(this, typeof c === "string" ? text(bevy, c) : c, null);
  });
  def("insertBefore", function (c: Node, b: Node | null) {
    return insert(this, c, b);
  });
  def("removeChild", function (c: Node) {
    return remove(c);
  });
  def("remove", function () {
    remove(this);
  });
  def("replaceChildren", function (...cs: (Node | string)[]) {
    [...this.childNodes].forEach((c) => remove(c));
    for (const c of cs) insert(this, typeof c === "string" ? text(bevy, c) : c, null);
  });
  def("setAttribute", function (name: string, v: unknown) {
    if (name === "style") return writeStyle(this, { ...Object.fromEntries(Object.keys(styleNow(this)).map((k) => [k, undefined])), ...parseCss(String(v)) });
    const k = PROP[name] ?? name;
    this.props = { ...this.props, [k]: String(v) };
    send({ op: "update", id: this.nodeId, kind: this.kind ?? this.type, props: { [k]: String(v) } });
  });
  def("removeAttribute", function (name: string) {
    const k = PROP[name] ?? name;
    const { [k]: _, ...rest } = this.props;
    this.props = rest;
    send({ op: "update", id: this.nodeId, kind: this.kind ?? this.type, props: {}, unset: [k] });
  });
  def("cloneNode", function (deep = false) {
    const c = element(bevy, this.type, { ...this.props, style: styleNow(this) });
    if (deep) for (const k of this.childNodes) (c as any).appendChild(isText(k) ? text(bevy, k.data) : (k as any).cloneNode(true));
    return c;
  });
  const set = (name: string, f: (this: El, v: any) => void) => {
    const d = Object.getOwnPropertyDescriptor(p, name);
    Object.defineProperty(p, name, { get: d?.get, set: f, configurable: true });
  };
  const setText = function (this: El, v: unknown) {
    [...this.childNodes].forEach((c) => remove(c));
    if (v != null && v !== "") insert(this, text(bevy, String(v)), null);
  };
  set("textContent", setText);
  set("innerText", setText);
  set("className", function (v) {
    (this as any).setAttribute("class", v);
  });
  set("id", function (v) {
    (this as any).setAttribute("id", v);
  });
}

function element(bevy: Bevy, tag: string, props: Record<string, unknown> = {}): El {
  const el = new bevy.BevyElement(next++, tag, tag, props) as El & Made;
  el.made = true;
  send({ op: "create", id: el.nodeId, kind: tag, props });
  return el;
}

function text(bevy: Bevy, data: string): Node {
  const t = new bevy.BevyTextNode(next++, data) as Node & Made;
  t.made = true;
  send({ op: "createText", id: t.nodeId, text: data });
  return t;
}

/** A CSS font shorthand's families ("500 100px 'Instrument Serif', serif" -> ["Instrument Serif", "serif"]). */
const families = (font: string) => (String(font).match(/\d[\d.]*(?:px|pt|em|rem|%)(?:\/\S+)?\s+(.+)$/)?.[1] ?? String(font)).split(",").map((f) => f.trim().replace(/^["']|["']$/g, "")).filter(Boolean);

class FontFace_ {
  status = "unloaded";
  readonly loaded: Promise<FontFace_>;
  private done!: (f: FontFace_) => void;
  constructor(readonly family: string, readonly source: unknown, readonly descriptors: Record<string, string> = {}) {
    this.loaded = new Promise((r) => (this.done = r));
  }
  /** A face given by URL is fetched now into the faces an svg's and a canvas's text are set in (fonts.face); one
   *  named only by family is the family the composition asked for (loadFont). One that doesn't load rejects, as in
   *  Chrome, so the text falls back. */
  load(): Promise<FontFace_> {
    const url = typeof this.source === "string" ? /url\(\s*["']?([^"')]+)["']?\s*\)/.exec(this.source)?.[1] : undefined;
    const faces = url ? native("fonts.face", { family: this.family, url }).faces : native("fonts.load", { family: this.family }).faces;
    if (url && !faces) {
      this.status = "error";
      return Promise.reject(new Error(`FontFace ${this.family}: ${url} didn't load`));
    }
    this.status = "loaded";
    this.done(this);
    return this.loaded;
  }
}

/** document.fonts: a family loads into the faces a canvas measures with (fonts.load); the page's own text has its
 *  faces before the frame is taken, so `ready` is settled. */
function fontSet() {
  const faces = new Set<FontFace_>();
  return {
    status: "loaded",
    ready: Promise.resolve(undefined as unknown),
    load(font: string): Promise<FontFace_[]> {
      for (const family of families(font)) native("fonts.load", { family });
      return Promise.resolve([...faces].filter((f) => families(font).includes(f.family)));
    },
    check: () => true,
    add(f: FontFace_) {
      faces.add(f);
      return this;
    },
    delete: (f: FontFace_) => faces.delete(f),
    has: (f: FontFace_) => faces.has(f),
    forEach: (cb: (f: FontFace_) => void) => faces.forEach(cb),
    get size() {
      return faces.size;
    },
    addEventListener() {},
    removeEventListener() {},
    [Symbol.iterator]: () => faces.values(),
  };
}

/** The document and window, once, from dom-reads. */
export function domDocument(bevy: Bevy): void {
  const g = globalThis as any;
  if (!bevy?.BevyElement || !bevy.containerNode || g.document || !HOST?.op_flush) return;
  writers(bevy);
  // the page's body: the composition's surface, the element the page draws from (raea-runtime-page's sync walks it),
  // found when first asked (the composition mounts after this runs); React's root container until it's there
  const root = bevy.containerNode(0);
  let surface: El | null = null;
  const bodyNow = (): El => {
    if (surface?.parentNode) return surface;
    const look = (e: El): El | null => {
      for (const c of e.childNodes ?? []) {
        const k = c as El;
        if ((k.kind ?? k.type) === "surface" && k.props?.id === "motion") return k;
        const f = k.childNodes ? look(k) : null;
        if (f) return f;
      }
      return null;
    };
    surface = look(root);
    return surface ?? root;
  };
  const html = {
    tagName: "HTML",
    nodeName: "HTML",
    nodeType: 1,
    get clientWidth() {
      return viewportSize()[0];
    },
    get clientHeight() {
      return viewportSize()[1];
    },
    get style() {
      return declaration(bodyNow());
    },
    get children() {
      return [bodyNow()];
    },
    getBoundingClientRect: () => (bodyNow() as any).getBoundingClientRect?.(),
    appendChild: (c: Node) => insert(bodyNow(), c, null),
    contains: (n: Node) => n === bodyNow() || (bodyNow() as any).contains?.(n),
  };
  const all = () => [bodyNow(), ...querySelectorAll(bodyNow(), "*")];
  const document = {
    nodeType: 9,
    nodeName: "#document",
    readyState: "complete",
    visibilityState: "visible",
    hidden: false,
    get body() {
      return bodyNow();
    },
    get head() {
      return bodyNow();
    },
    documentElement: html,
    fonts: fontSet(),
    defaultView: g,
    createElement: (tag: string) => (String(tag).toLowerCase() === "canvas" ? new OffscreenCanvas() : element(bevy, String(tag).toLowerCase())),
    createElementNS: (_: string, tag: string) => element(bevy, String(tag)),
    createTextNode: (data: string) => text(bevy, String(data)),
    createDocumentFragment: () => element(bevy, "div"),
    getElementById: (id: string) => all().find((e) => e.props?.id === id) ?? null,
    getElementsByClassName: (c: string) => querySelectorAll(bodyNow(), String(c).split(/\s+/).filter(Boolean).map((x) => `.${x}`).join("")),
    getElementsByTagName: (t: string) => querySelectorAll(bodyNow(), t),
    querySelector: (s: string) => querySelectorAll(bodyNow(), s)[0] ?? null,
    querySelectorAll: (s: string) => querySelectorAll(bodyNow(), s),
    contains: (n: Node) => n === bodyNow() || all().includes(n as El),
    addEventListener() {},
    removeEventListener() {},
  };
  domRange(document);
  g.document = document;
  g.window ??= g;
  g.FontFace ??= FontFace_;
  // the values the page computed for the element (LR5)
  g.getComputedStyle ??= (el: El) => computed(el, computedOf);
  for (const [k, f] of [
    ["innerWidth", () => viewportSize()[0]],
    ["innerHeight", () => viewportSize()[1]],
    ["devicePixelRatio", () => 1],
  ] as const) {
    if (!(k in g)) Object.defineProperty(g, k, { get: f, configurable: true });
  }
  g.addEventListener ??= () => {};
  g.removeEventListener ??= () => {};
}
