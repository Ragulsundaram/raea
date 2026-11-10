// 40 layers the size of the frame: more than Vello's image atlas holds at once (bench/layer-atlas/check.sh)
import { registerRoot } from "raea";
import { planes } from "./planes";

registerRoot(planes(40));
