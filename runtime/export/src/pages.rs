//! The frames' pages settled before raea-render reads them; each is written once as an asset by raea-runtime-pagestore.

use super::*;
pub(crate) use raea_runtime_pagestore::{pages_written, store_clear, store_page};

/// A frame's page collected from its paint and written as an asset (once per distinct page).
pub(crate) fn settle_page(frames: &mut [(i64, Frame)], k: usize, w: u32, h: u32, work: &Path) -> R<()> {
    if frames[k].1.same {
        frames[k].1.page = k.checked_sub(1).and_then(|j| frames[j].1.page.clone());
        if std::env::var_os("RAEA_KEEP").is_some() {
            if let Some(hash) = &frames[k].1.page {
                use std::io::Write as _;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(work.join("pages.txt")) {
                    let _ = writeln!(f, "{hash}");
                }
            }
        }
        return Ok(());
    }
    let _ = (w, h);
    Ok(())
}
