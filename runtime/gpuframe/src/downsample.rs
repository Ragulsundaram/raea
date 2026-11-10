//! A supersampled frame's readback shrunk to the frame's size (RAEA_SS, raea-render's target.rs), on the CPU.

/// sRGB bytes -> linear, as a table
fn srgb_to_linear_table() -> &'static [f32; 256] {
    static T: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| std::array::from_fn(|i| {
        let c = i as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    }))
}

/// an RGBA frame w x h shrunk by `s`: each s x s block's mean, in linear light (averaging the encoded bytes would darken
/// every edge between light and dark), encoded back to sRGB; alpha averaged as it is
pub fn downsample(px: &[u8], w: usize, h: usize, s: usize) -> Vec<u8> {
    let lin = srgb_to_linear_table();
    let (ow, oh) = (w / s, h / s);
    let n = (s * s) as f32;
    let mut out = vec![0u8; ow * oh * 4];
    for y in 0..oh {
        for x in 0..ow {
            let mut acc = [0f32; 4];
            for dy in 0..s {
                let row = ((y * s + dy) * w + x * s) * 4;
                for dx in 0..s {
                    let i = row + dx * 4;
                    acc[0] += lin[px[i] as usize];
                    acc[1] += lin[px[i + 1] as usize];
                    acc[2] += lin[px[i + 2] as usize];
                    acc[3] += px[i + 3] as f32;
                }
            }
            let o = (y * ow + x) * 4;
            for c in 0..3 {
                let v = acc[c] / n;
                let e = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
                out[o + c] = (e.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
            out[o + 3] = (acc[3] / n + 0.5) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_block_averages_in_linear_light() {
        // black and white side by side, 2x down: linear mid-grey (188 in sRGB), not the bytes' mean (128)
        let px = [0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255];
        assert_eq!(super::downsample(&px, 2, 2, 2), vec![188, 188, 188, 255]);
    }
}
