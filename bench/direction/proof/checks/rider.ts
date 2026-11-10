// A rider turns with its seat and is in no part of it (DR4's fifth proof: a 3D scene). The parts are named in the
// rule's values (`rider`; `seat`, and every part whose name starts with it is the seat's); their rotations are
// compared, and each part's own box, turned as it is, is tested against the rider's (raea/check's `inside`). That the
// rider rides the seat is DR16's Rides; until then the rule names them. DR6's worked check for "the seat turns with
// the computer" (sdk/guides/craft/checks/rides.ts) holds its place and turn on the seat as well.
import { check, found, inside, type Part } from "raea/check";

type V = number[];

/** The angle between two rotations (x, y, z, w), in degrees. */
const turn = (a: V, b: V) => (2 * Math.acos(Math.min(1, Math.abs(a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]))) * 180) / Math.PI;

export default check({ id: "rider", text: "a rider turns with its seat and is in no part of it" }, (world, _frame, values) => {
  const parts = world.find({ with: ["Part"] });
  const part = (id: number) => world.component<Part>(id, "Part")!;
  const rider = parts.find((p) => part(p).name === String(values.rider));
  const seat = parts.find((p) => part(p).name === String(values.seat));
  // a piece with neither has no rider to hold; one without the other is a name gone wrong
  if (rider == null && seat == null) return;
  if (rider == null || seat == null) return found({ says: `there's no part named ${rider == null ? values.rider : values.seat} on this frame` });
  const deg = turn(part(rider).rotation, part(seat).rotation);
  const most = Number(values.turn ?? 0.5);
  if (deg > most) found({ about: [rider, seat], says: `the rider is turned ${deg.toFixed(1)} degrees from its seat`, measure: { name: "turn", value: Math.round(deg * 10) / 10, unit: "deg", limit: most } });
  const into = Number(values.into ?? 0.01);
  for (const s of parts.filter((p) => p !== rider && part(p).name.startsWith(String(values.seat)))) {
    const cm = inside(part(rider), part(s));
    if (cm > into) found({ about: [rider, s], says: `the rider is ${Math.round(cm * 100)} cm into ${part(s).name}`, measure: { name: "into", value: Math.round(cm * 100), unit: "cm", limit: Math.round(into * 100) } });
  }
});
