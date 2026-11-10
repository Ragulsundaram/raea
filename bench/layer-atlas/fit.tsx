// 18 layers the size of the frame: Vello's image atlas has room for 28 (bench/layer-atlas/check.sh)
import { registerRoot } from "raea";
import { planes } from "./planes";

registerRoot(planes(18));
