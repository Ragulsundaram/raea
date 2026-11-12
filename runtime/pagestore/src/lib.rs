//! A render's frame pages written once each as assets (assets/HASH.png, straight alpha), on workers: the next frame
//! starts while one is written, and every page is written before raea-render reads them (raea-runtime-export).

use std::path::{Path, PathBuf};

type R<T> = Result<T, String>;

/// Pages being written on workers, and the paths already started (a repeated page is written once).
static WRITERS: std::sync::Mutex<Vec<std::thread::JoinHandle<R<()>>>> = std::sync::Mutex::new(Vec::new());
static PENDING: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> = std::sync::LazyLock::new(Default::default);

/// Every page written: before raea-render reads them.
pub fn pages_written() -> R<()> {
    let jobs: Vec<_> = std::mem::take(&mut *WRITERS.lock().unwrap());
    for j in jobs {
        j.join().map_err(|_| "a page writer panicked".to_string())??;
    }
    Ok(())
}

/// A PNG written for the engine to read straight back (video stills, pages): no filtering, the fastest deflate. The
/// default level spent 12 ms a page and as long again on every video still, for files that live a few seconds.
pub fn fast_png(path: &Path, rgba: &[u8], w: u32, h: u32) -> R<()> {
    use image::ImageEncoder;
    let f = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?);
    image::codecs::png::PngEncoder::new_with_quality(f, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::NoFilter)
        .write_image(rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())
}

/// A frame's page written once as an asset (assets/HASH.png, straight alpha), its hash; None when nothing is painted.
pub fn store_page(page: &[u8], w: u32, h: u32, work: &Path) -> R<Option<String>> {
    if page.iter().skip(3).step_by(4).all(|a| *a == 0) {
        return Ok(None);
    }
    // xxHash3 (a page is 8 MB: SipHash spent milliseconds on each one)
    let hash = format!("motion{:016x}", twox_hash::XxHash3_64::oneshot(page));
    // RAEA_KEEP: each frame's page in order, in pages.txt, to compare renders page by page
    if std::env::var_os("RAEA_KEEP").is_some() {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(work.join("pages.txt")) {
            let _ = writeln!(f, "{hash}");
        }
    }
    std::fs::create_dir_all(work.join("assets")).map_err(|e| e.to_string())?;
    let p = work.join("assets").join(format!("{hash}.png"));
    if !p.exists() && PENDING.lock().unwrap().insert(p.clone()) {
        // premultiplied to straight, as a PNG holds it; written on a worker so the next frame can start (WRITERS,
        // joined before raea-render reads the pages)
        let mut straight = page.to_vec();
        let path = p.clone();
        let job = std::thread::spawn(move || {
            for px in straight.chunks_exact_mut(4) {
                let a = px[3] as u32;
                if a > 0 && a < 255 {
                    for c in &mut px[..3] {
                        *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                    }
                }
            }
            fast_png(&path, &straight, w, h)
        });
        let mut ws = WRITERS.lock().unwrap();
        ws.push(job);
        if ws.len() > 8 {
            let first = ws.remove(0);
            drop(ws);
            first.join().map_err(|_| "a page writer panicked".to_string())??;
        }
    }
    Ok(Some(hash))
}

/// A fully clear picture `w` x `h` as an asset (store_page leaves a clear page out; a screen shows it).
pub fn store_clear(w: u32, h: u32, work: &Path) -> R<String> {
    let hash = format!("clear{w}x{h}");
    std::fs::create_dir_all(work.join("assets")).map_err(|e| e.to_string())?;
    let p = work.join("assets").join(format!("{hash}.png"));
    if !p.exists() {
        fast_png(&p, &vec![0u8; (w * h * 4) as usize], w, h)?;
    }
    Ok(hash)
}
