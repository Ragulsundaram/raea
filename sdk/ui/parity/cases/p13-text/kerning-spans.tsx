// Kerning across inline elements: a line whose letters are each their own span is as wide as the same line written
// once, as Chrome shapes a run of one font across element boundaries (snapcn's word-flip types its words a letter a
// span; raea's spans broke the shaping at each one, so "AV" lost its kern and the line grew: docs/layout-reads.feature
// LR10).
// proves: react-dom.element.span
// tier: quick
export const steps = [];
const TEXT = "AVAWATAVAWAT Portfolio";
export default function Case() {
  return (
    <div data-testid="root" className="h-full w-full bg-white p-4 text-slate-900" style={{ fontFamily: "Inter", fontSize: 48 }}>
      <div><span data-testid="plain" style={{ whiteSpace: "pre" }}>{TEXT}</span></div>
      <div><span data-testid="spans" style={{ whiteSpace: "pre" }}>{[...TEXT].map((c, i) => <span key={i}>{c}</span>)}</span></div>
      <div><span data-testid="coloured" style={{ whiteSpace: "pre" }}>{[...TEXT].map((c, i) => <span key={i} style={{ color: i % 2 ? "#be123c" : "#0f172a", opacity: 0.9 }}>{c}</span>)}</span></div>
    </div>
  );
}
