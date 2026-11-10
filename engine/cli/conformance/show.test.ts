// `raea show --on ITEM --working ID`: what a session sends the raea app's host for the person's comments (docs/review.feature
// R3), and where a note read from one stands (docs/direction.feature DR5, DR6): its asks, its badge and each ask's,
// and a clip under an ask for the person to sign. The host (apps/app/host/tasks.ts) is faked: this is the command's side.
import { describe, expect, test } from "bun:test";
import { showVerb } from "../src/show.ts";

/** showVerb with a host that keeps each op it was asked, and answers a task as the host does. */
function run(args: string[]) {
  const ops: [string, Record<string, unknown>][] = [];
  const ask = (op: string, b: Record<string, unknown>) => {
    ops.push([op, b]);
    if (op === "harness") return { name: "Claude Code", pid: 7 };
    return { item: b.item, project: "shop", comments: b.comments, state: b.state };
  };
  const out = showVerb(ask, args, "/work/shop", { CLAUDECODE: "1", CLAUDE_CODE_SESSION_ID: "abcdef123" });
  return { out, tasks: ops.filter(([op]) => op === "task").map(([, b]) => b) };
}

const HASH = "a".repeat(64);

describe("raea show --working, for a note", () => {
  test("a note to restate goes with its badge and no asks", () => {
    const { out, tasks } = run(["--on", "i-show-1", "--working", "cm-1", "--status", "to restate"]);
    expect(tasks).toHaveLength(1);
    expect(tasks[0]).toMatchObject({ item: "i-show-1", comments: ["cm-1"], state: "working", status: "to restate" });
    expect(tasks[0].asks).toBeUndefined();
    expect(out).toBe(`Marked cm-1 working on i-show-1: the person sees Claude Code on them; it reads "to restate".\n`);
  });

  test("its asks go with the note's badge and each ask's", () => {
    const { out, tasks } = run(["--on", "i-show-1", "--working", "cm-1", "--ask", "the lyric is flat type", "--ask", "it arrives as a lyric video's", "--status", "to check", "--states", "to check, failing"]);
    expect(tasks[0]).toMatchObject({ asks: ["the lyric is flat type", "it arrives as a lyric video's"], status: "to check", states: ["to check", "failing"] });
    expect(out).toContain(`it reads "to check" (its asks: to check, failing).`);
  });

  test("a clip to sign goes under its ask, its path made whole", () => {
    const { out, tasks } = run(["--on", "i-show-1", "--working", "cm-1", "--sign", "2", "--clip", ".raea/notes-clips/n1.2.mp4", "--render", HASH]);
    expect(tasks[0]).toMatchObject({ state: "working", sign: { ask: 2, clip: "/work/shop/.raea/notes-clips/n1.2.mp4", render: HASH } });
    expect(out).toBe(`Shown ask 2's clip on cm-1 of i-show-1 for the person to sign (render aaaaaaaaaaaa…): their sign comes to the inbox, and raea notes take keeps it.\n`);
  });

  test("a mark without these sends none of them, as before", () => {
    const { tasks } = run(["--on", "i-show-1", "--working", "cm-1", "--done", "cm-2"]);
    expect(tasks.map((t) => [t.state, t.status, t.states, t.sign])).toEqual([["working", undefined, undefined, undefined], ["done", undefined, undefined, undefined]]);
  });

  test("what doesn't fit is said before anything reaches the app", () => {
    const refused = (args: string[], says: string) => expect(() => run(["--on", "i-show-1", ...args])).toThrow(says);
    refused(["--done", "cm-1", "--status", "passing"], "--status, --states and --sign go with --working");
    refused(["--working", "cm-1", "--working", "cm-2", "--sign", "1", "--clip", "/c.mp4", "--render", HASH], "--sign N shows one comment's clip");
    refused(["--working", "cm-1", "--sign", "1", "--clip", "/c.mp4"], "--sign N shows one comment's clip");
    refused(["--working", "cm-1", "--sign", "1", "--clip", "/c.mp4", "--render", HASH, "--done", "cm-2"], "--sign N shows one comment's clip");
    refused(["--working", "cm-1", "--clip", "/c.mp4"], "--clip and --render go with --sign N");
    refused(["--working", "cm-1", "--ask", "one", "--states", "to check,failing"], "one state for each --ask: 1 asks, 2 states");
  });
});
