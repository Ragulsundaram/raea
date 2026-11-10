//! Whether the GPU still runs our work. A render's frame that came out all black and came out black again when drawn
//! once more is checked here before it's kept as the scene's own black (`black_checked`).
//! wgpu-hal's Metal backend counts a command buffer that ended in MTLCommandBufferStatus::Error (a GPU timeout or
//! hang, or work discarded in the recovery after one) as completed and reports nothing, so a fault there leaves a
//! frame's targets as they were, black, with no error anywhere.

use std::time::Duration;

/// On Metal, an empty command buffer of our own on `queue`, its status and error read (Metal discards a process's
/// work for a while after a hang, and says why); then a 4-byte copy through `queue`, read back. Err says what the GPU
/// didn't do.
pub fn canary(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
    if fault() >= 2 {
        return Err("RAEA_BLACK_FAULT=2 fails it".into());
    }
    #[cfg(target_vendor = "apple")]
    metal(queue)?;
    const MAGIC: u32 = 0x5241_4541;
    let usage = wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
    let src = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu canary"), size: 4, usage, mapped_at_creation: false });
    let dst = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu canary read"), size: 4, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    queue.write_buffer(&src, 0, &MAGIC.to_le_bytes());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gpu canary") });
    enc.copy_buffer_to_buffer(&src, 0, &dst, 0, 4);
    queue.submit([enc.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    dst.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(Duration::from_secs(20)) }).map_err(|e| format!("the GPU canary: {e}"))?;
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| format!("the GPU canary: {e}"))?;
    let got = {
        let data = dst.slice(..).get_mapped_range().map_err(|e| format!("the GPU canary: {e:?}"))?;
        u32::from_le_bytes([data[0], data[1], data[2], data[3]])
    };
    dst.unmap();
    if got != MAGIC {
        return Err(format!("the GPU didn't run a 4-byte copy (read {got:#010x}): it's discarding work"));
    }
    Ok(())
}

#[cfg(target_vendor = "apple")]
fn metal(queue: &wgpu::Queue) -> Result<(), String> {
    use objc2_metal::{MTLCommandBuffer, MTLCommandBufferStatus, MTLCommandQueue};
    let Some(hal) = (unsafe { queue.as_hal::<wgpu::hal::api::Metal>() }) else { return Ok(()) };
    let cb = hal.as_raw().commandBuffer().ok_or("Metal gave no command buffer")?;
    cb.commit();
    cb.waitUntilCompleted();
    if cb.status() == MTLCommandBufferStatus::Error {
        let why = cb.error().map_or_else(|| "no error given".to_string(), |e| format!("{e:?}"));
        return Err(format!("Metal ended a command buffer in error: {why}"));
    }
    Ok(())
}

/// Every pixel's colour 0 (RGBA rows): a frame drawn black, or clear.
pub fn black(px: &[u8]) -> bool {
    px.chunks_exact(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
}

/// RAEA_BLACK_FAULT, the black-frame redraw's check (sdk/raea/bench/black-frame): 1, each frame's first draw read
/// back black, as a GPU fault that leaves no error does; 2, every draw black and the canary failing too.
pub fn fault() -> u8 {
    match std::env::var("RAEA_BLACK_FAULT").as_deref() {
        Ok("1") => 1,
        Ok("2") => 2,
        _ => 0,
    }
}

/// `px` as the fault RAEA_BLACK_FAULT asks for leaves it: black (its alpha kept) on a first draw at 1, on any at 2.
pub fn injected(mut px: Vec<u8>, redraw: bool) -> Vec<u8> {
    if fault() > u8::from(redraw) {
        for p in px.chunks_exact_mut(4) {
            p[..3].fill(0);
        }
    }
    px
}

/// RAEA_PANIC_FAULT=N, the panicking-frame check (bench/frame-panic): a panic as frame N draws, as the page engine's
/// was in #24 (#33).
pub fn panic_fault(f: i64) {
    static AT: std::sync::OnceLock<Option<i64>> = std::sync::OnceLock::new();
    if *AT.get_or_init(|| std::env::var("RAEA_PANIC_FAULT").ok().and_then(|v| v.parse().ok())) == Some(f) {
        panic!("RAEA_PANIC_FAULT at frame {f}");
    }
}

/// A panic caught while frame `f` drew (`p`, catch_unwind's payload) as the render's error (#33): the frame and the
/// panic's message. The hook has printed the panic and where it was, so the render ends naming both, and its caller
/// keeps the frames it has.
pub fn panicked(p: Box<dyn std::any::Any + Send>, f: i64) -> String {
    let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "a panic with no message".into());
    format!("raea panicked drawing frame {f}: {msg}")
}

/// `draw`, frame `f`'s drawing, with a panic in it as the render's error (`panicked`) rather than the process's end.
pub fn frame_caught<T>(f: i64, draw: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic_fault(f);
        draw()
    }))
    .map_err(|p| panicked(p, f))?
}

static SCENE_BLACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether this render's scene is known to draw black: a black frame of it was drawn black twice and the GPU passed its
/// canary (`set` Some(true) from then on; Some(false) as a render begins). A later black frame is asked the canary
/// alone, not drawn again (about 2 s a frame in raea-render's careful path: an all-black render paid that on every
/// frame).
pub fn scene_black(set: Option<bool>) -> bool {
    match set {
        Some(v) => {
            SCENE_BLACK.store(v, std::sync::atomic::Ordering::Relaxed);
            v
        }
        None => SCENE_BLACK.load(std::sync::atomic::Ordering::Relaxed),
    }
}

static REFUSED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// A frame raea-page wouldn't draw (`bump_fits`), with the reason; `refused` hands it to the render reading the frame
/// back, which refuses the render with it (raea-render and sdk/react `black_checked`).
pub fn refuse(why: String) {
    *REFUSED.lock().unwrap_or_else(|e| e.into_inner()) = Some(why);
}

/// The reason a frame since the last call wasn't drawn, if one wasn't.
pub fn refused() -> Option<String> {
    REFUSED.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// A page draw Vello skipped (raea-page skipped.rs: past its bump buffers, with no error), found as its read back came
/// in, with what it says if Vello's own sizes skip it too.
static SKIPPED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
/// Frames still to draw again since a skipped draw was found: its answer comes in with the wait for some frame's
/// pixels, which may be that frame's or one drawn ahead of it (a render keeps up to 3 in flight: raea-render output.rs
/// frames_run), so the frame being read back and the 3 after it are drawn again.
static REDRAW_LEFT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const IN_FLIGHT: u32 = 3;

/// A page draw was skipped: `why` says so if it's skipped at Vello's own sizes too (the first one's kept until read).
pub fn skipped_draw(why: String) {
    SKIPPED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(why);
}

/// What makes the draws' read backs that are done answer (raea-page's non-blocking poll of its device, `on_settle`).
static SETTLE: std::sync::OnceLock<fn()> = std::sync::OnceLock::new();

/// Called before each frame's check: by then the frame's draws are done on the GPU, so their read backs answer.
pub fn on_settle(settle: fn()) {
    let _ = SETTLE.set(settle);
}

/// The fed render's frame `first`, checked before it goes to the encoder, where `skip_checked` could only draw the frames
/// after (a skipped first frame went out black): once `wait` (raea-page's `wait_drawn`) has every draw's read back
/// answered, a skipped draw has `paint` paint the frame again with Vello's own sizes; Err naming the frame when that's
/// skipped too (#23).
pub fn fed_checked<T>(f: i64, first: T, wait: fn(), redraw_full: fn(bool), paint: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    wait();
    // a frame raea-page refused (`refuse`) fails the render naming it, as a still's does (`black_checked`): the fed
    // render had no one to read the refusal, encoded the frame as it came and went on
    let unrefused = || refused().map_or(Ok(()), |why| Err(format!("frame {f}: {why}")));
    unrefused()?;
    if take_skipped().is_none() {
        return Ok(first);
    }
    redraw_full(true);
    let again = paint();
    redraw_full(false);
    let again = again?;
    wait();
    unrefused()?;
    match take_skipped() {
        Some(why) => Err(format!("frame {f}: {why}")),
        None => Ok(again),
    }
}

fn take_skipped() -> Option<String> {
    if let Some(settle) = SETTLE.get() {
        settle();
    }
    SKIPPED.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Frame `f`'s pixels, drawn again (`again`, every page with Vello's own sizes: `redraw_full`) when a page draw was skipped since the last
/// frame read back or in the frames just before it; Err naming the frame when the redraw is skipped too (#23). Every
/// render's readback calls it, through `black_checked` or (NV12) on its own.
pub fn skip_checked(f: i64, px: Vec<u8>, redraw_full: fn(bool), again: impl FnOnce() -> Result<Vec<u8>, String>) -> Result<Vec<u8>, String> {
    use std::sync::atomic::Ordering::Relaxed;
    if take_skipped().is_some() {
        REDRAW_LEFT.store(IN_FLIGHT + 1, Relaxed);
    }
    if REDRAW_LEFT.load(Relaxed) == 0 {
        return Ok(px);
    }
    REDRAW_LEFT.fetch_sub(1, Relaxed);
    // every page drawn again (a page painted as before isn't, else)
    redraw_full(true);
    let redrawn = again();
    redraw_full(false);
    let redrawn = redrawn?;
    if let Some(why) = take_skipped() {
        return Err(format!("frame {f}: {why}"));
    }
    Ok(redrawn)
}

/// Vello's seven bump buffers for a render of `w` x `h`, in bytes, at the sizes the render has (vello_encoding's
/// bump_sizes_for: grown with the target past 1080p).
pub fn bump_bytes(w: u32, h: u32) -> [(&'static str, u64); 7] {
    use vello::low_level::vello_encoding::{LineSoup, PathSegment, SegmentCount, Tile, bump_sizes_for};
    let s = bump_sizes_for(w.div_ceil(16) * h.div_ceil(16));
    let b = |n: u32, size: usize| n as u64 * size as u64;
    [
        ("lines", b(s.lines, size_of::<LineSoup>())),
        ("bin_data", b(s.bin_data, 4)),
        ("tiles", b(s.tiles, size_of::<Tile>())),
        ("seg_counts", b(s.seg_counts, size_of::<SegmentCount>())),
        ("segments", b(s.segments, size_of::<PathSegment>())),
        ("blend_spill", b(s.blend_spill, 4)),
        ("ptcl", b(s.ptcl, 4)),
    ]
}

/// Whether a `w` x `h` render's bump buffers fit `limit`, the bytes the device allows a storage buffer: their total
/// when they do, the reason when one doesn't. A render drawn into a buffer smaller than it needs overflows, and
/// non-robust Vello can hang the GPU then, so such a frame isn't drawn.
pub fn bump_fits(w: u32, h: u32, limit: u64) -> Result<u64, String> {
    let bytes = bump_bytes(w, h);
    match bytes.iter().find(|b| b.1 > limit) {
        Some((name, n)) => Err(format!("a {w}x{h} page needs a {} MB Vello {name} buffer, past the GPU's {} MB a buffer, so it isn't drawn (draw it smaller)", n >> 20, limit >> 20)),
        None => Ok(bytes.iter().map(|b| b.1).sum()),
    }
}

/// A render's peak memory footprint at `w` x `h`, estimated: a still of a text page measured 2.03 GB at 4K, 6.49 at 8K
/// and 14.46 at 12K (footprint, /usr/bin/time -l, 10-26), about 600 MB plus 192 bytes a pixel (48 RGBA copies of the
/// frame, Vello's bump buffers among them), which puts 16K at 25.8 GB, where memguard killed one. A floor: 3D,
/// video and pipelined frames add to it.
pub fn render_mem(w: u32, h: u32) -> u64 {
    (600 << 20) + 192 * w as u64 * h as u64
}

/// Whether a `w` x `h` render fits `cap` bytes (memguard's 24 GB a job unless RAEA_RENDER_MEM_CAP gives GB): the
/// reason not to start it when it doesn't, so it's refused before its first frame rather than killed in the middle.
pub fn mem_fits(w: u32, h: u32, cap: Option<u64>) -> Result<(), String> {
    let cap = cap.unwrap_or_else(|| std::env::var("RAEA_RENDER_MEM_CAP").ok().and_then(|v| v.trim_end_matches(['G', 'g']).parse::<f64>().ok()).map_or(24 << 30, |g| (g * (1u64 << 30) as f64) as u64));
    let need = render_mem(w, h);
    if need > cap {
        return Err(format!("a {w}x{h} render needs about {:.1} GB of memory, past the {:.1} GB a job may use (memguard; RAEA_RENDER_MEM_CAP=GB raises it): render it smaller", need as f64 / (1u64 << 30) as f64, cap as f64 / (1u64 << 30) as f64));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vello's own sizes here (no embedder's set in a test), wgpu's default 256 MiB a buffer
    #[test]
    fn bump_buffers_fit_or_refuse() {
        let limit = 256 << 20;
        let hd = bump_fits(1920, 1080, limit).unwrap();
        let uhd = bump_fits(3840, 2160, limit).unwrap();
        assert!(uhd > hd, "4K's buffers grow: {uhd} against {hd}");
        assert!(bump_fits(7680, 4320, 1 << 30).is_ok(), "8K fits a 1 GiB buffer");
        let e = bump_fits(15360, 8640, limit).unwrap_err();
        assert!(e.contains("15360x8640") && e.contains("isn't drawn"), "{e}");
    }

    // a skipped page draw (#23): the frame read back and the 3 in flight after it drawn again with every page redrawn;
    // a redraw skipped too refuses the frame by its number. One test, as the state is the process's.
    #[test]
    fn skipped_draws_are_drawn_again_or_refused() {
        static FULL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        fn full(on: bool) {
            if on {
                FULL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        assert_eq!(skip_checked(1, vec![1], full, || panic!("no draw skipped")).unwrap(), vec![1]);
        skipped_draw("past even Vello's sizes".into());
        for f in 2..6 {
            assert_eq!(skip_checked(f, vec![1], full, || Ok(vec![2])).unwrap(), vec![2], "frame {f} drawn again");
        }
        assert_eq!(FULL.load(std::sync::atomic::Ordering::Relaxed), 4, "every page redrawn each time");
        assert_eq!(skip_checked(6, vec![1], full, || panic!("past the frames in flight")).unwrap(), vec![1]);
        skipped_draw("past even Vello's sizes".into());
        let e = skip_checked(7, vec![1], full, || {
            skipped_draw("a page (1920x1080) wasn't drawn".into());
            Ok(vec![2])
        })
        .unwrap_err();
        assert_eq!(e, "frame 7: a page (1920x1080) wasn't drawn");
        // the fed render's frame, which no read back checks: a refused one fails the render by its number too, and
        // the frame after it isn't refused with it
        fn drawn() {}
        assert_eq!(fed_checked(8, 1, drawn, full, || panic!("no draw skipped")).unwrap(), 1);
        refuse("a page (1920x1080) was drawn without 2 of its images".into());
        let e = fed_checked(9, 1, drawn, full, || panic!("no draw skipped")).unwrap_err();
        assert_eq!(e, "frame 9: a page (1920x1080) was drawn without 2 of its images");
        assert_eq!(fed_checked(10, 1, drawn, full, || panic!("no draw skipped")).unwrap(), 1);
    }

    #[test]
    fn render_memory_fits_or_refuses() {
        let cap = Some(24 << 30);
        for (w, h) in [(1920, 1080), (3840, 2160), (7680, 4320), (11520, 6480)] {
            assert!(mem_fits(w, h, cap).is_ok(), "{w}x{h}");
        }
        // 16K: about 24.3 GiB (25.8 GB, where memguard killed one), refused before its first frame
        let e = mem_fits(15360, 8640, cap).unwrap_err();
        assert!(e.contains("15360x8640") && e.contains("24.0 GB"), "{e}");
        assert!(mem_fits(15360, 8640, Some(32 << 30)).is_ok());
    }
}

/// A frame that came out all black drawn once more (`again`), with Vello's own bump buffers (`redraw_full`, raea-page's):
/// a Vello overflow or a GPU fault (wgpu-hal's Metal backend doesn't report a command buffer that ended in error) draws
/// a frame black and says nothing. kitchen/water.tsx came out so under a loaded GPU, 10-26. Black again too, or black
/// once the render knows its scene draws black (`scene_black`), (true): the caller asks the GPUs (`black_twice`).
pub fn black_checked(f: i64, px: Vec<u8>, redraw_full: fn(bool), mut again: impl FnMut() -> Result<Vec<u8>, String>) -> Result<(Vec<u8>, bool), String> {
    if let Some(why) = refused() {
        return Err(format!("frame {f}: {why}"));
    }
    let px = skip_checked(f, px, redraw_full, &mut again)?;
    let px = injected(px, false);
    if !black(&px) {
        return Ok((px, false));
    }
    if scene_black(None) {
        return Ok((px, true));
    }
    redraw_full(true);
    let redrawn = again();
    redraw_full(false);
    let redrawn = injected(redrawn?, true);
    if black(&redrawn) {
        return Ok((redrawn, true));
    }
    eprintln!("raea: frame {f} came out all black and not when drawn again (Vello at its own bump sizes): the redraw is kept (a Vello bump overflow, or a GPU fault that left no error)");
    Ok((redrawn, false))
}

/// A frame black on two draws: the scene's own black when the GPUs still run work (their canaries), else refused.
pub fn black_twice(f: i64, canary: Result<(), String>) -> Result<(), String> {
    let known = scene_black(None);
    canary.map_err(|e| format!("frame {f} came out all black{}, and the GPU fails its canary: {e}", if known { "" } else { " on two draws" }))?;
    if !known {
        eprintln!("raea: frame {f} is all black on two draws and the GPU passes its canary: kept as the scene's black (later black frames are asked the canary alone)");
        scene_black(Some(true));
    }
    Ok(())
}
