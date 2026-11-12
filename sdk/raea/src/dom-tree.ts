// The tree on a composition's refs (docs/layout-reads.feature LR4): what a web component reads off an element to walk
// and query what it rendered. Answered in JS from bevy-react's instances (raea's patch keeps the tree React last
// committed on them), with no call to Rust. Installed from the entry with dom-reads.ts.
// Selectors: tag, *, #id, .class, [attr], [attr=v] (also ~= ^= $= *=), descendant and child combinators, comma lists,
// :first-child, :last-child, :only-child, :nth-child(an+b | odd | even) and :not(simple). Anything else throws a
// SyntaxError, as Chrome's does.

type Props = Record<string, unknown>;
export type Node = { nodeId: number; parentNode: El | null };
export type El = Node & { type: string; kind?: string; props: Props; childNodes: Node[] };
type Text = Node & { data: string };

const isEl = (n: unknown): n is El => !!n && Array.isArray((n as El).childNodes);
/** The root a tree is mounted in (bevy-react's container): not an element a component sees. */
const isRoot = (n: Node | null): boolean => !n || (isEl(n) && n.type === "root" && n.kind === "root");

/** Chrome's NodeList and HTMLCollection: an array with item(). */
function list<T>(a: T[]): T[] & { item(i: number): T | null } {
  return Object.assign(a, { item: (i: number) => a[i] ?? null });
}

const kids = (n: El) => n.childNodes.filter(isEl);
const siblings = (n: Node) => (n.parentNode ? n.parentNode.childNodes : [n]);

/** An attribute's value as the DOM has it from React's props (class from className, for from htmlFor). */
export function attr(el: El, name: string): string | null {
  const p = el.props;
  const key = name === "class" ? "className" : name === "for" ? "htmlFor" : name;
  let v = p[key] ?? (key !== name ? p[name] : undefined);
  if (name === "style" && v && typeof v === "object") v = Object.entries(v as Props).map(([k, x]) => `${k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase())}: ${x}`).join("; ");
  if (v === undefined || v === null || v === false || typeof v === "function" || name === "children" || name === "key" || name === "ref") return null;
  return v === true ? "" : String(v);
}

function textOf(n: Node): string {
  if (!isEl(n)) return (n as Text).data ?? "";
  if (n.childNodes.length === 0 && (typeof n.props.children === "string" || typeof n.props.children === "number")) return String(n.props.children);
  return n.childNodes.map(textOf).join("");
}

// ---- selectors

type Simple = { tag?: string; id?: string; classes: string[]; attrs: [string, string | null, string?][]; pseudos: ((e: El) => boolean)[] };
type Step = { simple: Simple; comb: " " | ">" | null };

function syntax(sel: string): never {
  const e = new Error(`Failed to execute 'querySelector' on 'Element': '${sel}' is not a valid selector.`);
  e.name = "SyntaxError";
  throw e;
}

/** an+b's test on a 1-based index */
function nth(arg: string, sel: string): (i: number) => boolean {
  const a = arg.replace(/\s+/g, "");
  if (a === "odd") return (i) => i % 2 === 1;
  if (a === "even") return (i) => i % 2 === 0;
  const m = /^([+-]?\d*)n([+-]\d+)?$/.exec(a);
  if (m) {
    const k = m[1] === "" || m[1] === "+" ? 1 : m[1] === "-" ? -1 : Number(m[1]);
    const b = Number(m[2] ?? 0);
    return (i) => (k === 0 ? i === b : (i - b) / k >= 0 && (i - b) % k === 0);
  }
  if (/^[+-]?\d+$/.test(a)) return (i) => i === Number(a);
  return syntax(sel);
}

const index = (e: El) => (e.parentNode ? kids(e.parentNode).indexOf(e) + 1 : 1);
const count = (e: El) => (e.parentNode ? kids(e.parentNode).length : 1);

/** One compound selector from `s` at `at`: its parts, and where it ended. */
function simple(s: string, at: number, sel: string): [Simple, number] {
  const out: Simple = { classes: [], attrs: [], pseudos: [] };
  const ident = /^-?[_a-zA-Z -￿][-_a-zA-Z0-9 -￿]*/;
  let i = at;
  const take = (re: RegExp) => {
    const m = re.exec(s.slice(i));
    if (!m) syntax(sel);
    i += m[0].length;
    return m;
  };
  if (s[i] === "*") i++;
  else if (ident.test(s.slice(i))) out.tag = take(ident)[0].toLowerCase();
  for (;;) {
    const c = s[i];
    if (c === "#") {
      i++;
      out.id = take(ident)[0];
    } else if (c === ".") {
      i++;
      out.classes.push(take(ident)[0]);
    } else if (c === "[") {
      const m = take(/^\[\s*([-_a-zA-Z0-9:]+)\s*(?:([~^$*]?=)\s*(?:"([^"]*)"|'([^']*)'|([^\]\s]+))\s*)?\]/);
      out.attrs.push([m[1], m[2] ? (m[3] ?? m[4] ?? m[5]) : null, m[2]]);
    } else if (c === ":") {
      const m = take(/^:([-a-z]+)(\(([^()]*(?:\([^()]*\))?[^()]*)\))?/);
      const [name, arg] = [m[1], m[3]];
      if (name === "first-child" && arg === undefined) out.pseudos.push((e) => index(e) === 1);
      else if (name === "last-child" && arg === undefined) out.pseudos.push((e) => index(e) === count(e));
      else if (name === "only-child" && arg === undefined) out.pseudos.push((e) => count(e) === 1);
      else if (name === "nth-child" && arg !== undefined) {
        const t = nth(arg, sel);
        out.pseudos.push((e) => t(index(e)));
      } else if (name === "not" && arg !== undefined) {
        const [inner, end] = simple(arg.trim(), 0, sel);
        if (end !== arg.trim().length) syntax(sel);
        out.pseudos.push((e) => !matchSimple(e, inner));
      } else syntax(sel);
    } else break;
  }
  if (i === at) syntax(sel);
  return [out, i];
}

/** A selector list: each selector's steps, right to left. */
function parse(sel: string): Step[][] {
  const lists: Step[][] = [];
  const s = sel.trim();
  if (!s) syntax(sel);
  let i = 0;
  let steps: Step[] = [];
  let comb: Step["comb"] = null;
  for (;;) {
    const [one, end] = simple(s, i, sel);
    steps.push({ simple: one, comb });
    i = end;
    const ws = /^\s*/.exec(s.slice(i))![0].length;
    const next = s[i + ws];
    if (next === undefined) break;
    if (next === ",") {
      lists.push(steps.reverse());
      steps = [];
      comb = null;
      i += ws + 1;
      i += /^\s*/.exec(s.slice(i))![0].length;
      continue;
    }
    if (next === ">") {
      comb = ">";
      i += ws + 1;
      i += /^\s*/.exec(s.slice(i))![0].length;
    } else if (ws > 0) {
      comb = " ";
      i += ws;
    } else syntax(sel);
  }
  lists.push(steps.reverse());
  return lists;
}

function matchSimple(e: El, s: Simple): boolean {
  if (s.tag && e.type.toLowerCase() !== s.tag) return false;
  if (s.id !== undefined && attr(e, "id") !== s.id) return false;
  if (s.classes.length) {
    const cls = (attr(e, "class") ?? "").split(/\s+/);
    if (!s.classes.every((c) => cls.includes(c))) return false;
  }
  for (const [name, want, op] of s.attrs) {
    const v = attr(e, name);
    if (v === null) return false;
    if (want === null || want === undefined) continue;
    const ok = op === "=" ? v === want : op === "~=" ? v.split(/\s+/).includes(want) : op === "^=" ? !!want && v.startsWith(want) : op === "$=" ? !!want && v.endsWith(want) : !!want && v.includes(want);
    if (!ok) return false;
  }
  return s.pseudos.every((p) => p(e));
}

/** Whether `e` matches a selector's steps (right to left), with ancestors no higher than `scope`'s root. */
function matchSteps(e: El, steps: Step[], k = 0): boolean {
  if (!matchSimple(e, steps[k].simple)) return false;
  if (k === steps.length - 1) return true;
  const comb = steps[k].comb;
  for (let p = e.parentNode; p && !isRoot(p); p = p.parentNode) {
    if (matchSteps(p, steps, k + 1)) return true;
    if (comb === ">") return false;
  }
  return false;
}

const cache = new Map<string, Step[][]>();
function compiled(sel: string): Step[][] {
  let c = cache.get(sel);
  if (!c) cache.set(sel, (c = parse(sel)));
  return c;
}

export function matches(e: El, sel: string): boolean {
  return compiled(sel).some((steps) => matchSteps(e, steps));
}

function descendants(e: El, out: El[] = []): El[] {
  for (const c of e.childNodes) if (isEl(c)) out.push(c), descendants(c, out);
  return out;
}

export function querySelectorAll(e: El, sel: string): El[] {
  const c = compiled(sel);
  return list(descendants(e).filter((d) => c.some((steps) => matchSteps(d, steps))));
}

/** LR4's tree on the element and text classes (bevy-react's BevyElement and BevyTextNode). */
export function domTree(Element: { prototype: object } | undefined, Text: { prototype: object } | undefined): void {
  if (!Element || (Element.prototype as any).__domTree) return;
  Object.defineProperty(Element.prototype, "__domTree", { value: true });
  const nodes = [Element.prototype, Text?.prototype].filter(Boolean) as object[];
  const get = (protos: object[], name: string, f: (this: any) => unknown) => protos.forEach((p) => Object.defineProperty(p, name, { get: f, configurable: true }));
  const method = (name: string, f: (this: any, ...a: any[]) => unknown) => Object.defineProperty(Element.prototype, name, { value: f, writable: true, configurable: true });
  const at = (n: Node, d: number, elements: boolean) => {
    const s = elements ? siblings(n).filter(isEl) : siblings(n);
    return s[s.indexOf(n as El) + d] ?? null;
  };
  // every node
  get(nodes, "parentElement", function () {
    return isRoot(this.parentNode) ? null : this.parentNode;
  });
  get(nodes, "nextSibling", function () {
    return at(this, 1, false);
  });
  get(nodes, "previousSibling", function () {
    return at(this, -1, false);
  });
  get(nodes, "nextElementSibling", function () {
    return at(this, 1, true);
  });
  get(nodes, "previousElementSibling", function () {
    return at(this, -1, true);
  });
  get(nodes, "ownerDocument", () => (globalThis as any).document ?? null);
  if (Text) {
    get([Text.prototype], "nodeType", () => 3);
    get([Text.prototype], "nodeName", () => "#text");
    get([Text.prototype], "textContent", function () {
      return this.data;
    });
    get([Text.prototype], "nodeValue", function () {
      return this.data;
    });
  }
  // elements
  const E = [Element.prototype];
  get(E, "nodeType", () => 1);
  get(E, "tagName", function () {
    return String(this.type).toUpperCase();
  });
  get(E, "nodeName", function () {
    return String(this.type).toUpperCase();
  });
  get(E, "localName", function () {
    return String(this.type);
  });
  get(E, "children", function () {
    return list(kids(this));
  });
  get(E, "childElementCount", function () {
    return kids(this).length;
  });
  get(E, "firstChild", function () {
    return this.childNodes[0] ?? null;
  });
  get(E, "lastChild", function () {
    return this.childNodes[this.childNodes.length - 1] ?? null;
  });
  get(E, "firstElementChild", function () {
    return kids(this)[0] ?? null;
  });
  get(E, "lastElementChild", function () {
    const k = kids(this);
    return k[k.length - 1] ?? null;
  });
  get(E, "id", function () {
    return attr(this, "id") ?? "";
  });
  get(E, "className", function () {
    return attr(this, "class") ?? "";
  });
  get(E, "classList", function () {
    const all = (attr(this, "class") ?? "").split(/\s+/).filter(Boolean);
    return Object.assign(list([...all]), { value: all.join(" "), contains: (c: string) => all.includes(c), toString: () => all.join(" ") });
  });
  get(E, "dataset", function () {
    const out: Record<string, string> = {};
    for (const k of Object.keys(this.props)) if (k.startsWith("data-")) out[k.slice(5).replace(/-([a-z])/g, (_, c: string) => c.toUpperCase())] = attr(this, k) ?? "";
    return out;
  });
  get(E, "textContent", function () {
    return textOf(this);
  });
  get(E, "innerText", function () {
    return textOf(this);
  });
  method("getAttribute", function (name: string) {
    return attr(this, name);
  });
  method("hasAttribute", function (name: string) {
    return attr(this, name) !== null;
  });
  method("hasChildNodes", function () {
    return this.childNodes.length > 0;
  });
  method("contains", function (other: Node | null) {
    for (let n = other; n; n = n.parentNode) if (n === this) return true;
    return false;
  });
  method("matches", function (sel: string) {
    return matches(this, sel);
  });
  method("closest", function (sel: string) {
    for (let n: El | null = this; n && !isRoot(n); n = n.parentNode) if (matches(n, sel)) return n;
    return null;
  });
  method("querySelector", function (sel: string) {
    return querySelectorAll(this, sel)[0] ?? null;
  });
  method("querySelectorAll", function (sel: string) {
    return querySelectorAll(this, sel);
  });
  method("getElementsByTagName", function (tag: string) {
    return querySelectorAll(this, tag === "*" ? "*" : tag);
  });
  method("getElementsByClassName", function (names: string) {
    return querySelectorAll(this, names.trim().split(/\s+/).map((c) => "." + c).join(""));
  });
}
