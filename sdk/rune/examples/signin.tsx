// A sign-in card styled only by Tailwind classes, for both targets (docs/targets.feature T5a): fields that ring on
// focus, a checkbox that shows a focus-visible ring when Tab reaches it, a submit that stays disabled until the form is
// ready, md: putting the two fields side by side, and dark: for the system's appearance. The submit springs on "snap"
// in Rune; on the web that is a CSS transition (@raea/rune's web.css).
import { useState } from "react";
import { rune } from "@raea/rune";

const field =
  "h-9 w-full rounded-md border border-zinc-300 bg-white px-3 text-sm text-zinc-900 outline-none hover:border-zinc-400 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/30 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-50";

function Field({ id, name, value, set }: { id: string; name: string; value: string; set: (v: string) => void }) {
  return (
    <div data-testid={`${id}-row`} className="flex flex-1 flex-col gap-1.5">
      <span className="text-sm font-medium text-zinc-700 dark:text-zinc-300">{name}</span>
      <input data-testid={id} value={value} onChange={(e) => set((e.target as HTMLInputElement).value)} className={field} />
    </div>
  );
}

export default function SignIn() {
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [remember, setRemember] = useState(false);
  const [signedIn, setSignedIn] = useState(false);
  const ready = email.includes("@") && code.length === 6;
  return (
    <div data-testid="page" className="flex min-h-screen flex-col items-center bg-zinc-50 p-6 dark:bg-zinc-950">
      <div data-testid="card" className="flex w-full max-w-lg flex-col gap-5 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
        <span data-testid="title" className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
          {signedIn ? `Signed in as ${email}` : "Sign in"}
        </span>
        <div data-testid="fields" className="flex flex-col gap-4 md:flex-row">
          <Field id="email" name="Email" value={email} set={setEmail} />
          <Field id="code" name="Six-digit code" value={code} set={setCode} />
        </div>
        <div
          data-testid="remember"
          role="checkbox"
          aria-checked={remember}
          tabIndex={0}
          onClick={() => setRemember(!remember)}
          className="flex flex-row items-center gap-2 rounded-md p-1 hover:bg-zinc-100 focus-visible:ring-2 focus-visible:ring-blue-500 dark:hover:bg-zinc-800"
        >
          <div data-testid="box" className={`size-4 rounded border ${remember ? "border-blue-600 bg-blue-600" : "border-zinc-400 bg-white dark:border-zinc-600"}`} />
          <span className="text-sm text-zinc-700 dark:text-zinc-300">Remember this device</span>
        </div>
        <button
          data-testid="submit"
          disabled={!ready}
          onClick={ready ? () => setSignedIn(true) : undefined}
          className="h-9 rounded-md bg-zinc-900 px-4 text-sm font-medium text-white hover:bg-zinc-700 disabled:opacity-50 dark:bg-zinc-50 dark:text-zinc-900"
          {...rune({ motion: "snap" })}
        >
          Sign in
        </button>
      </div>
    </div>
  );
}

export const steps = [
  { type: "ada@example.com", into: "email" },
  { type: "123456", into: "code" },
  { key: "Tab" },
  { press: "remember" },
  { press: "submit" },
];
