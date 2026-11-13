// el.style on a ref (docs/layout-reads.feature LR5): a CSSStyleDeclaration over the element's style prop. Reading it
// answers the style React last committed with the composition's own writes over it; writing a property (assignment,
// setProperty, removeProperty or cssText) sends an update op for that element, as canvas2d.ts sends a canvas's draws,
// so the page lays it out and draws it. React's next commit to the same property wins, as it does in Chrome, where
// React DOM writes only what changed: a write is forgotten once React's own update to that property lands.

type El = { nodeId: number; type: string; kind?: string; props: Record<string, unknown> };
type Css = Record<string, unknown>;

const HOST = (globalThis as any).__bevyHost;
/** The composition's writes over each element's committed style, by node id. */
const writes = new Map<number, Css>();
const decls = new WeakMap<object, unknown>();
/** a write of our own going through op_flush (forget leaves it) */
let ours = false;

// React DOM's unitless properties: a number is written as it is, any other gets px
const UNITLESS = new Set(["opacity", "zIndex", "fontWeight", "lineHeight", "flex", "flexGrow", "flexShrink", "order", "zoom", "scale", "aspectRatio", "columnCount", "gridRow", "gridColumn", "orphans", "widows", "fillOpacity", "strokeOpacity", "strokeWidth", "tabSize"]);
export const camel = (k: string) => (k.startsWith("--") ? k : k.replace(/^-ms-/, "ms-").replace(/-([a-z])/g, (_, c: string) => c.toUpperCase()));
export const kebab = (k: string) => (k.startsWith("--") ? k : k.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`));
/** A style value as Chrome's style declaration answers it: a string, px on a length React wrote as a number. */
export const cssValue = (k: string, v: unknown) => (v == null || v === false ? "" : typeof v === "number" && !UNITLESS.has(k) && v !== 0 ? `${v}px` : String(v));

/** The element's style as it stands: React's committed style with the composition's writes over it. */
export function styleNow(el: El): Css {
  const own = (el.props?.style ?? {}) as Css;
  const w = writes.get(el.nodeId);
  if (!w) return own;
  const out = { ...own, ...w };
  for (const k in w) if (w[k] === undefined) delete out[k];
  return out;
}

/** Writes `set` (camelCase; undefined unsets) to the element now, outside React's commits. */
export function writeStyle(el: El, set: Css): void {
  const w = writes.get(el.nodeId) ?? {};
  writes.set(el.nodeId, w);
  const style: Css = {};
  const unset: string[] = [];
  for (const k in set) {
    w[k] = set[k];
    if (set[k] === undefined || set[k] === "") unset.push(k), (w[k] = undefined);
    else style[k] = set[k];
  }
  const op: Record<string, unknown> = { op: "update", id: el.nodeId, kind: el.kind ?? el.type, props: Object.keys(style).length ? { style } : {} };
  if (unset.length) op.styleUnset = unset;
  ours = true;
  try {
    HOST.op_flush(JSON.stringify([op]), false);
  } finally {
    ours = false;
  }
}

/** React's own update to a property the composition wrote takes it back (React's commit wins). */
function forget(json: string): void {
  if (ours || !writes.size || !json.includes('"update"')) return;
  for (const op of JSON.parse(json) as { op: string; id?: number; props?: { style?: Css }; styleUnset?: string[] }[]) {
    const w = op.op === "update" && op.id !== undefined ? writes.get(op.id) : undefined;
    if (!w) continue;
    for (const k of [...Object.keys(op.props?.style ?? {}), ...(op.styleUnset ?? [])]) delete w[k];
    if (!Object.keys(w).length) writes.delete(op.id!);
  }
}

/** cssText's declarations, camelCase. */
export function parseCss(text: string): Css {
  const out: Css = {};
  for (const d of String(text).split(";")) {
    const at = d.indexOf(":");
    if (at > 0) out[camel(d.slice(0, at).trim())] = d.slice(at + 1).replace(/!important\s*$/, "").trim();
  }
  return out;
}

/** The element's CSSStyleDeclaration. */
export function declaration(el: El): unknown {
  const was = decls.get(el);
  if (was) return was;
  const now = () => styleNow(el);
  const methods: Record<string, unknown> = {
    getPropertyValue: (n: string) => cssValue(camel(n), now()[camel(n)]),
    getPropertyPriority: () => "",
    setProperty: (n: string, v: unknown) => writeStyle(el, { [camel(n)]: v == null ? "" : String(v) }),
    removeProperty: (n: string) => {
      const old = cssValue(camel(n), now()[camel(n)]);
      writeStyle(el, { [camel(n)]: undefined });
      return old;
    },
    item: (i: number) => kebab(Object.keys(now())[i] ?? ""),
  };
  const d = new Proxy(methods, {
    get(t, k) {
      if (typeof k !== "string") return undefined;
      if (k in t) return t[k];
      if (k === "length") return Object.keys(now()).length;
      if (k === "cssText") return Object.entries(now()).map(([p, v]) => `${kebab(p)}: ${cssValue(p, v)};`).join(" ");
      if (/^\d+$/.test(k)) return kebab(Object.keys(now())[Number(k)] ?? "");
      return cssValue(camel(k), now()[camel(k)]);
    },
    set(_, k, v) {
      if (typeof k !== "string") return false;
      if (k === "cssText") {
        const next = parseCss(String(v));
        const clear: Css = {};
        for (const p in now()) if (!(p in next)) clear[p] = undefined;
        writeStyle(el, { ...clear, ...next });
      } else writeStyle(el, { [camel(k)]: v == null ? "" : v });
      return true;
    },
  });
  decls.set(el, d);
  return d;
}

/** getComputedStyle(el) (LR5): the values the page computed for the element (`page.layout`'s styles, read when first
 *  asked and kept until the next commit), and for a property the page doesn't answer, the element's own style. Read
 *  only, as Chrome's is. */
export function computed(el: El, of: (id: number) => Record<string, string> | undefined): unknown {
  const value = (n: string) => {
    const k = kebab(n);
    const page = of(el.nodeId);
    return page?.[k] ?? cssValue(camel(n), styleNow(el)[camel(n)]);
  };
  const keys = () => Object.keys(of(el.nodeId) ?? {});
  const methods: Record<string, unknown> = { getPropertyValue: value, getPropertyPriority: () => "", item: (i: number) => keys()[i] ?? "" };
  return new Proxy(methods, {
    get(t, k) {
      if (typeof k !== "string") return undefined;
      if (k in t) return t[k];
      if (k === "length") return keys().length;
      if (k === "cssText") return "";
      if (/^\d+$/.test(k)) return keys()[Number(k)] ?? "";
      return value(k);
    },
    set: () => true,
  });
}

/** `style` on the element class (bevy-react's BevyElement), once, from dom-reads. */
export function domStyle(Element: { prototype: object } | undefined): void {
  if (!Element || Object.getOwnPropertyDescriptor(Element.prototype, "style")) return;
  Object.defineProperty(Element.prototype, "style", {
    configurable: true,
    get(this: El) {
      return declaration(this);
    },
    set(this: El, v: unknown) {
      (declaration(this) as { cssText: string }).cssText = String(v);
    },
  });
  if (HOST?.op_flush) {
    const flush = HOST.op_flush;
    HOST.op_flush = (json: string, devtools: boolean) => {
      forget(json);
      return flush(json, devtools);
    };
  }
}
