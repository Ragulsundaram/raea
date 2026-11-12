//! Layout reads (docs/layout-reads.feature LR1, #54): the composition's `page.layout` answered by the page that draws its
//! frame, synchronously. The JS thread asks and blocks; settle, on the main thread, wakes on the ask as it wakes on a
//! flush, applies the commits already sent, and has its host lay the page out and answer every element's boxes at once.
//! A host that lays nothing out answers that it can't, and the composition's read throws that.
//! Each runtime has its own ask (one process runs several: a bake's workers, a render's pre-pass beside its frames), so
//! a read is answered only by its own runtime's settle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::{Resource, World};

use crate::scene::HANG_SECS;

/// A host's answer to a layout read: the page laid out as the React World has it now, its snapshot (raea-sys-page's,
/// with offsets, keyed by React id) as JSON; or why it can't.
pub type Answer<'a> = dyn FnMut(&mut World) -> Result<String, String> + 'a;

/// One runtime's layout read: asked by its JS thread, answered by its settle.
#[derive(Default)]
pub struct Ask {
    /// the JS thread is waiting on a read
    asked: AtomicBool,
    /// the read's answer, once settle has made it
    answer: Mutex<Option<Result<String, String>>>,
    told: Condvar,
}

impl Ask {
    /// Whether a read is waiting (wait_flush wakes on it).
    pub fn asked(&self) -> bool {
        self.asked.load(Ordering::SeqCst)
    }
}

/// The runtime's ask, in its App (ReactPlugin puts it there and on its JS thread).
#[derive(Resource, Clone, Default)]
pub struct LayoutAsk(pub Arc<Ask>);

thread_local! {
    /// The JS thread's runtime's ask.
    pub(crate) static ASK: std::cell::RefCell<Option<Arc<Ask>>> = const { std::cell::RefCell::new(None) };
}

/// settle's side: a read waiting is answered by `answer`, on this (the main) thread.
pub(crate) fn answer_if_asked(ask: &Ask, world: &mut World, answer: &mut Answer) {
    if !ask.asked.swap(false, Ordering::SeqCst) {
        return;
    }
    let t0 = Instant::now();
    let r = answer(world);
    READS.fetch_add(1, Ordering::Relaxed);
    READ_NS.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
    *ask.answer.lock().unwrap() = Some(r);
    ask.told.notify_all();
}

/// Layout reads answered, and the nanoseconds the main thread spent answering them (LR10's cost per read)
pub static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static READ_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The JS thread's side (`page.layout`): ask, wake settle's wait, and block until the answer comes.
fn read() -> Result<String, String> {
    let ask = ASK.with(|a| a.borrow().clone()).ok_or("page.layout: this thread runs no composition")?;
    let mut g = ask.answer.lock().unwrap();
    *g = None;
    ask.asked.store(true, Ordering::SeqCst);
    // settle's wait is on FLUSHED's condvar: notified under its lock, so a wait about to start can't miss it
    {
        let (n, fcv) = &crate::FLUSHED;
        let _n = n.lock().unwrap();
        fcv.notify_all();
    }
    let (mut g, r) = ask.told.wait_timeout_while(g, Duration::from_secs_f64(HANG_SECS), |a| a.is_none()).unwrap();
    if r.timed_out() {
        ask.asked.store(false, Ordering::SeqCst);
        return Err(format!("page.layout got no answer in {HANG_SECS} s: the host never stepped the composition while it waited"));
    }
    g.take().unwrap_or_else(|| Err("page.layout: no answer".into()))
}

/// The native call (thread.rs's register_natives).
pub(crate) fn register() {
    raea_native::register(
        "page.layout",
        "{}",
        "string",
        "every element's boxes as the page that draws the frame lays it out now (docs/layout-reads.feature LR1): JSON {vw, vh, n: [id, x, y, w, h, scroll x y w h, padding l t r b, border l t r b, z per element], o: [id, offsetLeft, offsetTop, offsetWidth, offsetHeight, offsetParent, inline per element]}, keyed by React id; the composition's thread waits for it",
        |_| read().map(serde_json::Value::String),
    );
}

/// The answer of a pass that runs the composition but draws nothing (the sound pass, the 3D rows' pre-pass): no element
/// has a box, so every read is 0, and the composition goes on as it would before it's laid out.
pub const NOTHING_DRAWN: &str = "{\"vw\":0,\"vh\":0,\"dpr\":1,\"n\":[],\"o\":[]}";

/// [`NOTHING_DRAWN`] as a pass's answer.
pub fn nothing_drawn() -> impl FnMut(&mut World) -> Result<String, String> {
    |_| Ok(NOTHING_DRAWN.into())
}

/// A host's answer when it has no page to lay out.
pub fn no_page(host: &'static str) -> impl FnMut(&mut World) -> Result<String, String> {
    move |_| Err(format!("{host} lays no page out, so a composition can't read its layout here (getBoundingClientRect, offsetWidth and the rest)"))
}
