// A frame's 3D read from the page (docs/web-export.md M3): every `data-3d` element the SDK's components rendered, as
// the tree raea-web reads (engine/web/src/scene.rs), in tree order. Each element keeps an id while it lives, and its
// props and shape text go only when they changed since they were last sent: a ground's vertex buffer crosses once.

export interface Part {
  k: string;
  i: number;
  p?: string | null;
  s?: string | null;
  c?: Part[];
}

export class Collector {
  private ids = new WeakMap<Element, number>();
  private sent = new WeakMap<Element, { p: string | null; s: string | null }>();
  private next = 1;

  /** `pixel`: the engine draws at 1/pixel of the composition's size (PlayOptions.pixel) and the canvas scales it up with
   *  hard pixels, so a <Look>'s cell (pixelSize, in the composition's pixels) is that many fewer of the drawn ones: a
   *  pixel look of 4 drawn at a quarter size is one cell a drawn pixel, and the GPU shades a sixteenth of the pixels. */
  constructor(private pixel = 1) {}

  /** The page's 3D parts this frame. */
  parts(page: Element): Part[] {
    const out: Part[] = [];
    this.visit(page, out);
    return out;
  }

  private visit(el: Element, into: Part[]) {
    for (const c of el.children) {
      if (c.hasAttribute("data-3d")) {
        const part = this.part(c);
        into.push(part);
        const kids: Part[] = [];
        this.visit(c, kids);
        if (kids.length) part.c = kids;
      } else this.visit(c, into);
    }
  }

  private part(el: Element): Part {
    let i = this.ids.get(el);
    if (i === undefined) this.ids.set(el, (i = this.next++));
    let p = el.getAttribute("data-props");
    if (p && this.pixel !== 1 && el.getAttribute("data-3d") === "Look") p = this.scaled(p);
    const s = el.getAttribute("data-shape");
    const was = this.sent.get(el);
    const part: Part = { k: el.getAttribute("data-3d")!, i };
    if (!was || was.p !== p) part.p = p;
    if (!was || was.s !== s) part.s = s;
    this.sent.set(el, { p, s });
    return part;
  }

  private looks = new Map<string, string>();

  /** a Look's props with its cell in drawn pixels (1 at the least) */
  private scaled(p: string): string {
    let out = this.looks.get(p);
    if (out === undefined) {
      const o = JSON.parse(p) as { pixelSize?: number };
      if (typeof o.pixelSize === "number") o.pixelSize = Math.max(1, o.pixelSize / this.pixel);
      out = JSON.stringify(o);
      this.looks.set(p, out);
    }
    return out;
  }
}

const UNDER = "data-raea-under3d";

/** The 3D is drawn under the page: the page elements holding it are cleared, and the innermost solid background among
 *  them becomes the 3D's clear colour, as the native engine shows the page's background behind the 3D. */
export function clearUnder3d(page: HTMLElement): string | undefined {
  const first = page.querySelector("[data-3d]");
  if (!first) return undefined;
  let clear: string | undefined;
  for (let el = first.parentElement; el && el !== page; el = el.parentElement) {
    if (!el.hasAttribute(UNDER)) {
      const bg = getComputedStyle(el).backgroundColor;
      if (bg && bg !== "transparent" && !bg.endsWith(", 0)")) el.setAttribute(UNDER, hex(bg));
      else continue;
    }
    clear ??= el.getAttribute(UNDER)!;
  }
  return clear;
}

/** rgb(r, g, b) as #rrggbb (the engine's colours are hex) */
function hex(css: string): string {
  const m = css.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
  return m ? "#" + [m[1], m[2], m[3]].map((n) => Number(n).toString(16).padStart(2, "0")).join("") : css;
}

export const UNDER3D_CSS = `[${UNDER}]{background:transparent!important}`;
