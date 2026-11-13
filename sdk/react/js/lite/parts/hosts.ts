// Host nodes made, patched and dropped. Each host keeps the node a ref to it holds (bevy-react's BevyElement, its
// BevyCanvasElement for a <canvas>, a BevyTextNode for a text) linked into the tree bevy-react keeps under React, so a
// composition played live reads its refs as it does when rendered (docs/layout-reads.feature LR2).
import { allocId, buildUpdateOp, createCanvasElement, dropHandlers, push, serializeProps, ROOT_ID } from "../../node_modules/bevy-react/src/bridge";
import { BevyElement, BevyTextNode, containerNode } from "../../node_modules/bevy-react/src/nodes";
import { Host, Instance, Out, Slot, same } from "./all";

function kindOf(type: string, inText: boolean) {
  return !inText ? type : type === "text" ? "textSpan" : type === "image" ? "inlineImage" : type;
}

/** The named components an element was rendered in, nearest first, up to three ("Paddle Court Pong"). */
function ownerOf(inst: Instance | null): string {
  let names = "";
  for (let n = 0; inst && n < 3; inst = inst.parent) {
    const name = inst.fn?.displayName ?? inst.fn?.name;
    if (typeof name !== "string" || !name) continue;
    names = n++ ? `${names} ${name}` : name;
  }
  return names;
}

/** A ref given `node` (null to let go), a callback or an object, as React sets one. */
function setRef(ref: any, node: unknown) {
  if (typeof ref === "function") ref(node);
  else if (ref && typeof ref === "object") ref.current = node;
}

function create(o: Out): Host {
  const id = allocId();
  if (o.type === "#text") {
    push(o.inText ? { op: "createTextSpan", id, text: o.text } : { op: "createText", id, text: o.text });
    return { id, type: "#text", kind: "#text", text: true, props: null, value: o.text, kids: [], slot: o.slot, out: o, node: new BevyTextNode(id, o.text ?? "") };
  }
  const kind = kindOf(o.type, o.inText);
  const props = serializeProps(id, o.props, o.type);
  // the components it was rendered in, nearest first: what the feed's play stage names a picked element by
  if (o.owner && props) props["data-raea-owner"] = ownerOf(o.owner);
  push({ op: "create", id, kind, props, text: o.text });
  let node: BevyElement;
  if (o.type === "canvas") {
    node = createCanvasElement(id);
    node.props = o.props;
  } else node = new BevyElement(id, o.type, kind, o.props);
  const h: Host = { id, type: o.type, kind, text: false, props: o.props, value: o.text, kids: [], slot: o.slot, out: o, node };
  for (const k of o.kids) {
    const c = create(k);
    push({ op: "append", parent: id, child: c.id });
    h.kids.push(c);
    node.childNodes.push(c.node);
    c.node.parentNode = node;
  }
  if (o.ref != null) setRef(o.ref, node);
  return h;
}

function drop(h: Host) {
  dropHandlers(h.id);
  if (h.out?.ref != null) setRef(h.out.ref, null);
  for (const k of h.kids) drop(k);
}

function patch(h: Host, o: Out) {
  if (h.out === o) return;
  const was = h.out;
  h.out = o;
  if (h.text) {
    if (h.value !== o.text) push({ op: "updateText", id: h.id, text: o.text });
    h.value = o.text;
    (h.node as BevyTextNode).data = o.text ?? "";
    return;
  }
  const op = buildUpdateOp(h.id, h.props, o.props, h.type, h.kind);
  if (op) push(op);
  if (o.text !== undefined && o.text !== h.value) push({ op: "updateText", id: h.id, text: o.text });
  else if (o.text === undefined && h.value !== undefined && h.type === "text") push({ op: "updateText", id: h.id, text: "" });
  h.value = o.text;
  h.props = o.props;
  (h.node as BevyElement).props = o.props;
  h.kids = children(h.id, h.kids, o.kids, h.node as BevyElement);
  // a ref that changed lets the old one go and gives the new one the node, as React does
  if (was?.ref !== o.ref) {
    if (was?.ref != null) setRef(was.ref, null);
    if (o.ref != null) setRef(o.ref, h.node);
  }
}

/** A host parent's children matched to what the render yields: same place and type kept (patched), the rest made or
 * removed, and moved where their order changed. `node` is the parent's node, whose children follow. */
export function children(parent: number, old: Host[], next: Out[], node: BevyElement = containerNode(ROOT_ID)): Host[] {
  // the usual frame: the same places in the same order, each patched where it is (no maps, nothing moves)
  if (old.length === next.length) {
    let same = true;
    for (let i = 0; i < old.length; i++) {
      const h = old[i];
      const o = next[i];
      if (h.slot !== o.slot || h.type !== o.type || !(h.text || h.kind === kindOf(o.type, o.inText))) {
        same = false;
        break;
      }
    }
    if (same) {
      for (let i = 0; i < old.length; i++) patch(old[i], next[i]);
      return old;
    }
  }
  const bySlot = new Map<Slot, Host>();
  for (const h of old) bySlot.set(h.slot, h);
  const kept = new Set<Host>();
  const out: Host[] = next.map((o) => {
    const h = bySlot.get(o.slot);
    if (h && h.type === o.type && !kept.has(h) && (h.text || h.kind === kindOf(o.type, o.inText))) {
      kept.add(h);
      patch(h, o);
      return h;
    }
    return create(o);
  });
  for (const h of old) {
    if (!kept.has(h)) {
      push({ op: "remove", parent, child: h.id });
      drop(h);
      if (h.node.parentNode === node) h.node.parentNode = null;
    }
  }
  // the parent's node holds them in their new order
  node.childNodes = out.map((h) => h.node);
  for (const h of out) h.node.parentNode = node;
  // order: walk from the end, each one in place before the next (only those out of place move)
  const pos = new Map<Host, number>();
  old.forEach((h, i) => kept.has(h) && pos.set(h, i));
  let lastOld = Infinity;
  for (let i = out.length - 1; i >= 0; i--) {
    const h = out[i];
    const was = pos.get(h);
    if (was !== undefined && was < lastOld) {
      lastOld = was;
      continue;
    }
    const before = out[i + 1];
    push(before ? { op: "insert", parent, child: h.id, before: before.id } : { op: "append", parent, child: h.id });
  }
  return out;
}
