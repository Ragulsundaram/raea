// The page's own background behind its 3D: the first painted background among the 3D's enclosing elements, which
// raea's engine clears to (the AbsoluteFill a scene sits in). Read once an element (computed style is slow), and from
// its inline style when the page isn't in the document (<Raea>'s page is off screen).
/** set on each element read; a host hides those backgrounds so the 3D shows through (sdk/playable's stylesheet) */
export const UNDER = "data-raea-under3d";

export function backgroundUnder3d(page: Element): string | undefined {
  const first = page.querySelector("[data-3d]");
  if (!first) return undefined;
  let clear: string | undefined;
  for (let el = first.parentElement; el && el !== page; el = el.parentElement) {
    if (!el.hasAttribute(UNDER)) {
      const style = (el as HTMLElement).style;
      const bg = el.isConnected && typeof getComputedStyle === "function" ? getComputedStyle(el).backgroundColor : style?.backgroundColor || style?.background;
      if (bg && bg !== "transparent" && !bg.endsWith(", 0)")) el.setAttribute(UNDER, bg);
      else continue;
    }
    clear ??= el.getAttribute(UNDER)!;
  }
  return clear;
}
