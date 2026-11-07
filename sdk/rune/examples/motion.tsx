// R6a's example: a toast list that springs in Rune and renders as a plain page anywhere else. A press adds a toast
// that slides in on "pop"; a badge counts them with rolling digits; the panel behind has glass; a switch toggles
// whether new toasts are urgent.
import { useState } from "react";
import { Badge, RollingText, Spring, Switch, rune } from "@raea/rune";

export default function Toasts() {
  const [toasts, setToasts] = useState<{ id: number; urgent: boolean }[]>([]);
  const [urgent, setUrgent] = useState(false);
  const add = () => setToasts((t) => [...t, { id: t.length + 1, urgent }]);
  return (
    <div data-testid="panel" style={{ display: "flex", flexDirection: "column", gap: 8, width: 320, padding: 16, backgroundColor: "#15171c" }} {...rune({ glass: true })}>
      <div style={{ display: "flex", flexDirection: "row", gap: 8, alignItems: "center" }}>
        <button data-testid="add" onClick={add} style={{ padding: 8, borderRadius: 8, backgroundColor: "#2563eb", color: "#fff" }} {...rune({ motion: "snap" })}>
          Add
        </button>
        <span style={{ color: "#f9fafb" }}>
          <RollingText text={`${toasts.length}`} motion="snap" />
        </span>
        <Badge label={urgent ? "urgent" : "normal"} variant={urgent ? "accent" : "subtle"} dark />
        <Switch checked={urgent} onChange={setUrgent} label="Urgent" dark />
      </div>
      {toasts.map((t) => (
        <div
          key={t.id}
          data-testid={`toast-${t.id}`}
          style={{ padding: 12, borderRadius: 10, backgroundColor: t.urgent ? "#7f1d1d" : "#23262d", color: "#f9fafb" }}
          {...rune({ transition: { effect: { push: "top" }, enter: "pop", exit: Spring(0.2) }, lead: { lead: "snap", trail: "settle" } })}
        >
          {`Toast ${t.id}`}
        </div>
      ))}
    </div>
  );
}

export const steps = [{ press: "add" }, { press: "add" }, { press: "add" }];
