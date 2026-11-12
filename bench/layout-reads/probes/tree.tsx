// LR4's reads as a probe (docs/layout-reads.feature): a ref walked and queried as a web component does, each answer
// printed as words, so the harness's pairing by words fails on any answer that differs from Chrome's.
import { AbsoluteFill } from "raea";
import { loadFont } from "raea/text";
import { useLayoutEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 360, fps: 30, durationInFrames: 3 };
export const props = {};

export function Comp() {
  const list = useRef<HTMLUListElement>(null);
  const [lines, setLines] = useState<string[]>([]);
  useLayoutEffect(() => {
    const l = list.current!;
    const items = l.querySelectorAll("li");
    const second = l.querySelector("li:nth-child(2)") as HTMLElement;
    const word = l.querySelector(".word[data-w='b']") as HTMLElement;
    let bad = "no throw";
    try {
      l.querySelector("li[");
    } catch (e) {
      bad = (e as Error).name;
    }
    setLines([
      `tag ${l.tagName} kids ${l.children.length} items ${items.length} first ${l.firstElementChild?.textContent} last ${l.lastElementChild?.textContent}`,
      `second ${second.textContent} next ${second.nextElementSibling?.textContent} prev ${second.previousElementSibling?.textContent}`,
      `word ${word.textContent} closest ${word.closest("li")?.id} parent ${word.parentElement?.tagName} data ${word.dataset.w}`,
      `matches ${second.matches("ul > li.item:not(.off)")} odd ${l.querySelectorAll("li:nth-child(odd)").length} last ${l.querySelectorAll("li:last-child").length}`,
      `class ${word.className} list ${word.classList.length} has ${word.classList.contains("word")} attr ${word.getAttribute("data-w")} ${word.hasAttribute("title")}`,
      `contains ${l.contains(word)} ${word.contains(l)} text ${l.textContent?.replace(/\s+/g, "-")} bad ${bad}`,
    ]);
  }, []);
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily, fontSize: 16 }}>
      <ul ref={list} style={{ position: "absolute", left: 24, top: 10, margin: 0 }}>
        <li id="one" className="item">
          one
        </li>
        <li id="two" className="item">
          two <span className="word" data-w="b">bee</span>
        </li>
        <li id="three" className="item off">
          three
        </li>
      </ul>
      <div style={{ position: "absolute", left: 24, top: 140 }}>
        {lines.map((t) => (
          <div key={t}>{t}</div>
        ))}
      </div>
    </AbsoluteFill>
  );
}
