// loadFont (raea/text): a Google Fonts family for the page, as a stylesheet <link> loads it in Chrome. Inside raea the
// family's Latin faces (each weight and style Google has) are fetched when it's called, once, through raea's URL cache
// (raea-sys-fonts google.rs), so a canvas or svg measure in the first effect already has them, a render stays a
// function of its code, and it plays offline once cached. Families
// nobody asked for aren't fetched: Chrome loads only what a page links (RAEA_PAGE_GOOGLE_FONTS=1 fetches every family
// a page names). In a browser it links Google's stylesheet, so the same code shows the same family there.
import { hasNative, native } from "@raea/app/native";

// every weight, upright and italic: Google answers with the ones the family has (as google.rs asks)
const WEIGHTS = "100,100i,200,200i,300,300i,400,400i,500,500i,600,600i,700,700i,800,800i,900,900i";
const asked = new Set<string>();

/** The page gets Google Fonts' `family` ("Inter", "Space Grotesk"); `fontFamily` is the name to style text with. */
export function loadFont(family: string): { fontFamily: string } {
  const name = family.trim();
  if (!name || asked.has(name)) return { fontFamily: name };
  asked.add(name);
  if (hasNative()) {
    native("fonts.google", { family: name });
  } else if (typeof document !== "undefined" && document.head) {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = `https://fonts.googleapis.com/css?family=${encodeURIComponent(name).replace(/%20/g, "+")}:${WEIGHTS}&display=block`;
    document.head.appendChild(link);
  }
  return { fontFamily: name };
}
