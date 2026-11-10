---
title: "Apps on the web target or native"
summary: "Write a desktop app as one React file, pick the web target (Blitz) or the native target (Rune), check it headless with scripted steps, and bundle it as a Mac app."
order: 3
sources: ["docs/targets.feature", "apps/targets/src", "sdk/ui/src/app.ts", "sdk/rune/src", "sdk/rune/examples"]
---

An app is a React file with a default export: DOM elements, with inline `style` or Tailwind `className`, on raea's
UI layer. One setting picks the host that draws it. Neither uses the operating system's widgets or a browser; both draw
every pixel themselves on the GPU.

| | web (the default) | native |
|---|---|---|
| Engine | Blitz on Bevy, a browser engine | Rune, an app UI framework |
| CSS | All of it, grid included | A subset: flex, no grid |
| npm UI packages | Web ones work | Those that stay in the subset |
| 3D | Yes | Not yet |
| Motion | CSS transitions | Springs on every change, GPU effects |
| Accessibility tree | | AccessKit, for screen readers |
| Host size | about 175 MB | about 27 MB |

Pick native for a small, motion-heavy app with no 3D that passes its check. Anything else stays on web. A video or a
game is a composition, not an app.

## A sign-in card

This app ships with raea as `sdk/rune/examples/signin.tsx` and passes the check on both targets. It uses only
Tailwind classes:

```tsx
import { useState } from "react";
import { rune } from "@raea/rune";

export default function SignIn() {
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const ready = email.includes("@") && code.length === 6;
  return (
    <div data-testid="page" className="flex min-h-screen flex-col items-center bg-zinc-50 p-6">
      <div className="flex w-full max-w-lg flex-col gap-5 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
        <span className="text-lg font-semibold text-zinc-900">Sign in</span>
        <input data-testid="email" value={email} onChange={(e) => setEmail((e.target as HTMLInputElement).value)}
          className="h-9 rounded-md border border-zinc-300 px-3 text-sm focus:ring-2 focus:ring-blue-500/30" />
        <input data-testid="code" value={code} onChange={(e) => setCode((e.target as HTMLInputElement).value)}
          className="h-9 rounded-md border border-zinc-300 px-3 text-sm focus:ring-2 focus:ring-blue-500/30" />
        <button data-testid="submit" disabled={!ready}
          className="h-9 rounded-md bg-zinc-900 px-4 text-sm font-medium text-white disabled:opacity-50"
          {...rune({ motion: "snap" })}>
          Sign in
        </button>
      </div>
    </div>
  );
}
```

(This is a shortened copy; the shipped file adds a "remember this device" checkbox, a side-by-side layout from `md:`
up, and dark mode.)

Unlike a composition, an app is interactive, so `useState` and event handlers work as in any React app.

## Pick the target

Say the target in the app's own file, as a literal:

```tsx
export const app = { target: "native", title: "Notes", size: [480, 640], minSize: [360, 480] };
```

With no `app` export, or no `target` in it, the target is `"web"`. `--target web` or `--target native` on the command
line overrides it.

## Run, check and bundle

```sh
raea app run signin.tsx                       # opens it in a window
raea app check signin.tsx --png out/          # runs it headless and writes a PNG after each step
raea app bundle signin.tsx -o SignIn.app      # makes a Mac app
```

Give the check its input as steps in the app's file, finding elements by `data-testid`:

```tsx
export const steps = [{ press: "email" }, { type: "ada@example.com", into: "email" }, { type: "123456", into: "code" }, { press: "submit" }];
```

or as `--script steps.json`. The check writes `00-mount.png`, then a PNG after each step, and `tree.json` with each
element's role, label, text and box. Read the PNGs. When the app should run on both targets, check both:

```sh
raea app check signin.tsx --target web --png out/web/
raea app check signin.tsx --target native --png out/native/
```

What each target is held to:

- **native** fails on any prop Rune doesn't map and any host call it doesn't answer, naming each with its element's
  path in `unmapped.json` and `unanswered.json`. Change the prop, or keep the app on web.
- **web** passes when it draws, and lists what native wouldn't draw.

On a native app, `raea app check --video out/app.mp4` films the steps at 60 fps, with a cursor gliding to each element
it presses.

## Signing

`raea app bundle` signs ad hoc unless you give it your own identity:

```sh
raea app bundle signin.tsx -o SignIn.app --identity "Developer ID Application: Your Name (TEAMID)" --notarize PROFILE
```

`PROFILE` is a `notarytool` keychain profile. raea always signs with your identity, never its own.

## What only Rune does

Typed props from `@raea/rune` add Rune's motion and effects:

```tsx
<div {...rune({ motion: "pop", transition: { effect: { push: "top" }, enter: "pop" } })}>...</div>
```

There are also `lead`, `instant`, `blur`, `backdropBlur` and `glass`, and the components `Badge`, `Spinner`,
`Progress`, `RollingText` and `Switch`. On the web target a preset motion becomes a CSS transition, blur and glass
become filters, and the rest is ignored and listed by the check.

## Host calls

`call` and `callSync` from `@raea/ui/wire` reach the host on both targets: `net.fetch`, `net.ws.*`, `net.storage.*`,
`net.clipboard.*`, `os.open` (http, https and mailto), `fs.pick`, `fs.save`, `fs.read`, `fs.write` and
`dialog.message`. In a headless check, a step answers a dialog: `{ pick: ["a.txt"] }`, `{ save: "out.txt" }`,
`{ dialog: 0 }`.
