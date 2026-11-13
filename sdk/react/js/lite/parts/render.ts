// Rendering a tree, and letting go of what it no longer reaches.
import { Context, EMPTY, Fragment, Instance, Out, PROVIDER, S, Slot, codeLog, codeOf, inner, isValidElement, item, named, readLog, readsSame, renderedCodes, same, sameRun, slot } from "./all";

export function render(node: any, parent: Instance, slot: Slot, out: Out[], inText: boolean) {
  if (node === undefined || node === null || typeof node === "boolean") return;
  if (Array.isArray(node)) {
    for (let i = 0; i < node.length; i++) {
      const c = node[i];
      render(c, parent, item(slot, isValidElement(c) && c.key !== null ? c.key : i), out, inText);
    }
    return;
  }
  if (typeof node === "string" || typeof node === "number") {
    out.push({ slot, type: "#text", props: null, text: String(node), kids: EMPTY, inText });
    return;
  }
  if (!isValidElement(node)) return;
  const t = node.type;
  if (t === Fragment) {
    render(node.props.children, parent, slot, out, inText);
    return;
  }
  if (typeof t === "object" && t && t.$$provider === PROVIDER) {
    const ctx = t.ctx as Context<any>;
    const had = S.contexts.has(ctx._id);
    const prev = S.contexts.get(ctx._id);
    S.contexts.set(ctx._id, node.props.value);
    render(node.props.children, parent, slot, out, inText);
    if (had) S.contexts.set(ctx._id, prev);
    else S.contexts.delete(ctx._id);
    return;
  }
  if (typeof t === "function") {
    S.components++;
    const code = codeOf(t);
    renderedCodes.add(code);
    const k = named(slot, t.name || "c");
    let inst = parent.kids.get(k);
    if (!inst || inst.fn !== t) {
      if (inst) unmountInstance(inst);
      inst = { fn: t, hooks: [], effects: [], cleanups: new Map(), kids: new Map(), seen: true, id: k, parent, el: null, inText, out: null, reads: null, codes: null, stale: false, misses: 0, cool: 0, sites: null, stamp: 0 };
      parent.kids.set(k, inst);
    }
    inst.seen = true;
    const same = node === inst.el && inst.inText === inText;
    if (inst.cool > 0) inst.cool--;
    if (S.bailout && same && inst.out && !inst.stale) {
      if (readsSame(inst.reads!)) {
        // its output is what it made last time: it isn't called (nor anything under it), and its effects don't run
        // (as React.memo)
        const o = inst.out, r = inst.reads!, c = inst.codes!;
        for (let i = 0; i < o.length; i++) out.push(o[i]);
        for (let i = 0; i < r.length; i++) readLog[S.readN++] = r[i];
        for (let i = 0; i < c.length; i++) {
          codeLog[S.codeN++] = c[i];
          renderedCodes.add(c[i]);
        }
        inst.misses = 0;
        return;
      }
      if (++inst.misses >= 3) {
        inst.cool = 64;
        inst.misses = 0;
      }
    }
    const keep = same && inst.cool === 0;
    const outAt = out.length, readAt = S.readN, codeAt = S.codeN;
    codeLog[S.codeN++] = code;
    const savedCurrent = S.current;
    const savedIndex = S.hookIndex;
    S.current = inst;
    S.hookIndex = 0;
    inst.stamp = ++S.stampSerial;
    let rendered;
    try {
      rendered = t.prototype && t.prototype.render ? new t(node.props).render() : t(node.props);
    } finally {
      S.current = savedCurrent;
      S.hookIndex = savedIndex;
    }
    // (the loops skipped for a leaf: each for-of over a Map makes an iterator, for hundreds of components a frame)
    const had = inst.kids.size > 0;
    if (had) inst.kids.forEach(unsee);
    render(rendered, inst, k, out, inText);
    if (had || inst.kids.size > 0) {
      inst.kids.forEach(sweep, inst.kids);
    }
    inst.el = node;
    inst.inText = inText;
    inst.stale = false;
    if (keep) {
      inst.out = out.slice(outAt);
      inst.reads = sameRun(inst.reads, readLog, readAt, S.readN) ? inst.reads : readLog.slice(readAt, S.readN);
      inst.codes = sameRun(inst.codes, codeLog, codeAt, S.codeN) ? inst.codes : codeLog.slice(codeAt, S.codeN);
    } else {
      inst.out = null;
    }
    effectsQueue[S.effectsN++] = inst;
    return;
  }
  if (typeof t === "string") {
    // the same element at the same place, with no component under it, yields the same output: reused whole (and
    // patch skips it, h.out === o). Not on the passes that render everything (the bailout's), so every place under it
    // stays reached
    if (node._o !== null && S.bailout && node._s === slot && node._t === inText) {
      out.push(node._o);
      return;
    }
    const before = S.components;
    const childText = t === "text";
    const c = node.props.children;
    const inline = t === "text" && (typeof c === "string" || typeof c === "number");
    // an element with no children shares one empty list (each frame made one for every leaf)
    const kids: Out[] = inline || c === undefined || c === null ? EMPTY : [];
    if (kids !== EMPTY) render(c, parent, inner(slot), kids, inText || childText);
    const o: Out = { slot, type: t, props: node.props, kids, inText, text: inline ? String(c) : undefined, owner: parent, ref: node.ref };
    out.push(o);
    if (S.components === before) {
      node._o = o;
      node._s = slot;
      node._t = inText;
    }
  }
}

// A component's children marked unreached before it renders, and those still unreached after let go. By forEach
// with these, made once: a for-of made an iterator each time, and over the entries a [key, value] array for every
// component under it, every frame (lightspeed Z3)
export function unsee(kid: Instance) {
  kid.seen = false;
}

export function sweep(this: Map<Slot, Instance>, kid: Instance) {
  if (!kid.seen) {
    unmountInstance(kid);
    this.delete(kid.id as Slot);
  }
}

export const effectsQueue: Instance[] = [];

function unmountInstance(inst: Instance) {
  for (const c of inst.cleanups.values()) {
    try {
      c();
    } catch (e) {
      console.error("[js] an effect's cleanup threw:", e);
    }
  }
  inst.cleanups.clear();
  for (const k of inst.kids.values()) unmountInstance(k);
  inst.kids.clear();
}
