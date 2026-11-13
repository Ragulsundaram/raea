//! Stepping the App until the composition reaches a frame (its marker), or says it hangs.

use super::*;

/// Every schedule of `app` on the single-threaded executor: its systems hold the page (DomMirror, not Send) and run on
/// this thread anyway, and the multi-threaded one spawned a task per system a frame (ZeroHeap)
pub fn one_thread(app: &mut App) {
    let mut schedules = app.world_mut().resource_mut::<Schedules>();
    for (_, s) in schedules.iter_mut() {
        s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    }
}

/// The page's elements with their kind, props and node, as a frame reads them: one query kept in the world (ZeroHeap:
/// `world.query` made a QueryState each call, twice a live frame)
#[derive(Resource)]
pub struct KindQuery(pub QueryState<(&'static ReactKind, Option<&'static ReactProps>, Option<&'static crate::ReactNode>)>);

/// `f` with the world and its kept KindQuery
pub fn with_kinds<T>(world: &mut World, f: impl FnOnce(&mut World, &mut KindQuery) -> T) -> T {
    if !world.contains_resource::<KindQuery>() {
        let q = world.query();
        world.insert_resource(KindQuery(q));
    }
    world.resource_scope(|world, mut q: Mut<KindQuery>| f(world, &mut q))
}

/// The least a settings' settle (frame -1) waits: a hang guard for a module's top-level work, not a budget.
pub const SETTINGS_SECS: f64 = 120.0;

/// One JS call running longer than this is a hang (a loop that never returns), not progress.
pub const HANG_SECS: f64 = 600.0;

/// The longest a frame may be held (delayRender, a suspended scene) with nothing else moving.
pub const HOLD_SECS: f64 = 120.0;

/// Run React until its commits settle and the marker says `want` (or anything, for None). A layout read the composition
/// makes meanwhile is told this host lays no page out: a host with a page settles with [`settle_with`].
pub fn settle(app: &mut App, want: Option<i64>, secs: f64) -> R<Option<Map<String, Value>>> {
    settle_with(app, want, secs, &mut crate::layout::no_page("this host"))
}

/// [`settle`], with `answer` laying the host's page out for a layout read the composition makes while it runs
/// (`page.layout`, docs/layout-reads.feature LR1): the read wakes the wait as a flush does, the commits sent before it
/// are applied, and `answer` lays the page out from the World and returns its snapshot.
pub fn settle_with(app: &mut App, want: Option<i64>, secs: f64, answer: &mut crate::layout::Answer) -> R<Option<Map<String, Value>>> {
    // the settings' settle waits on the module's own top-level work too: parkour lays and performs its courses there,
    // 17-18 s of CPU on a quiet machine, and at load 20 the callers' 20 s refused it every time (10-26)
    let secs = if want == Some(-1) { secs.max(SETTINGS_SECS) } else { secs };
    // no polling for quiet: each pass applies what React flushed, then waits for its next flush. The composition marks
    // a frame done (data-raea-done) once its commit and that commit's effects have run, so the frame is taken the
    // moment the mark lands (it waited for three quiet updates with 1 ms sleeps, ~4 ms a frame)
    // the last time anything moved, and what it was (settle's no-progress guard, below)
    let mut still = std::time::Instant::now();
    let mut moved = (u64::MAX, u64::MAX, f64::NAN);
    // the marker as last seen (frame, done), for the refusal's message
    let mut marked = (-2i64, -2i64);
    // the frame is held (below): its marker at the frame, not done
    let mut held;
    // the count before the first update: a flush is in the channel before it's counted, so one that landed before this
    // read is applied by the update below, and one after it wakes the wait. (A first pass that waited for a flush
    // before updating lost the frame's flush whenever React was quicker than this read: the wait then ran its 0.5 s
    // out, once per launch in the legacy app's bake and as 0.5 s hitches in live play; lightspeed F5, F6.)
    let mut seen = crate::flushed();
    // the page styled at the frame's time: its CSS animations and transitions where they are at frame `want`
    // a bake's runtime (OneBatch): only what a commit needs, applied and its animations sampled (PreUpdate). The whole
    // main schedule ran each frame for nothing else, ~50 us a frame, more than React's render (lightspeed T3)
    let pre_only = app.world().get_resource::<crate::OneBatch>().is_some();
    // this runtime's layout reads (layout.rs): a replay's App has none
    let ask = app.world().get_resource::<crate::layout::LayoutAsk>().map(|a| a.0.clone());
    loop {
        if pre_only {
            app.world_mut().run_schedule(PreUpdate);
        } else {
            app.update();
        }
        if let Some(e) = crate::COMPOSITION_ERROR.lock().unwrap().take() {
            return Err(format!("the composition threw at frame {}: {e}", want.map(|w| w.to_string()).unwrap_or("?".into())));
        }
        // a layout read waiting: every commit sent before it applied (one an update with OneBatch), then answered, and
        // the loop goes on (the read's thread carries on from its effect, so more commits and the marker follow)
        if let Some(ask) = ask.as_deref().filter(|a| a.asked()) {
            // (the read's own commit can land after this pass's update: its batch is sent before its effects run)
            app.world_mut().run_schedule(PreUpdate);
            while app.world().get_resource::<crate::OneBatch>().is_some_and(|q| !q.0.is_empty()) {
                app.world_mut().run_schedule(PreUpdate);
            }
            crate::layout::answer_if_asked(ask, app.world_mut(), answer);
            continue;
        }
        let world = app.world_mut();
        // the marker's entity, remembered (a scan of every entity's props for it each pass was most of a settle)
        let marker = world.get_resource::<MarkerEntity>().map(|m| m.0).filter(|e| world.get::<ReactProps>(*e).is_some_and(|p| p.0.contains_key("data-raea-frame")));
        let marker = match marker {
            Some(e) => Some(e),
            None => {
                let mut q = world.query::<(Entity, &ReactProps)>();
                if let Some(e) = q.iter(world).find_map(|(_, p)| p.0.get("data-raea-error").and_then(|v| v.as_str()).map(str::to_string)) {
                    return Err(format!("the composition threw at frame {}: {e}", want.map(|w| w.to_string()).unwrap_or("?".into())));
                }
                let found = q.iter(world).find(|(_, p)| p.0.contains_key("data-raea-frame")).map(|(e, _)| e);
                if let Some(e) = found {
                    world.insert_resource(MarkerEntity(e));
                }
                found
            }
        };
        if let Some(m) = marker.and_then(|e| world.get::<ReactProps>(e)) {
            if let Some(e) = m.0.get("data-raea-error").and_then(|v| v.as_str()) {
                return Err(format!("the composition threw at frame {}: {e}", want.map(|w| w.to_string()).unwrap_or("?".into())));
            }
            let at = num(&m.0, "data-raea-frame").unwrap_or(-2.0) as i64;
            let done = num(&m.0, "data-raea-done").unwrap_or(-2.0) as i64;
            marked = (at, done);
            let ok = match want {
                // the settings: the first marker, before any frame
                None => true,
                Some(w) if w < 0 => at == w,
                Some(w) => at == w && done == w,
            };
            if ok {
                // the settings, read for the settings' settle only (a frame's settle returns None: every caller of one
                // ignores it, and parsing them each frame was most of a settle's allocations; ZeroHeap)
                if want.is_some_and(|w| w >= 0) {
                    return Ok(None);
                }
                let conf = m.0.get("data-raea-config").and_then(|v| v.as_str()).and_then(|s| serde_json::from_str(s).ok());
                return Ok(conf);
            }
        }
        if let Some(e) = crate::BUNDLE_ERROR.lock().unwrap().take() {
            return Err(format!("the composition threw before its first frame: {e}"));
        }
        // refused only when nothing moves for `secs`: no commit flushed, no JS run, nothing made on first use (a
        // matchmove solve: minutes). A call the JS thread is still in counts too, up to HANG_SECS (a frame's render
        // under load runs seconds before its first flush). A total-time limit refused film.tsx under load (10-27)
        let now = (crate::flushed(), crate::JS_NS.load(std::sync::atomic::Ordering::Relaxed), track::making_secs());
        let in_call = crate::js_call_for().is_some_and(|d| d.as_secs_f64() < HANG_SECS);
        if now != moved || in_call {
            moved = now;
            still = std::time::Instant::now();
        }
        // a held frame (delayRender, a scene still loading: its marker at the frame, not done) waits up to
        // HOLD_SECS, so a hold's own timeout (30 s unless it says) fails the render naming it first (layout-reads LR9)
        held = want.is_some_and(|w| marked.0 == w && marked.1 != w);
        let left = if held { secs.max(HOLD_SECS) } else { secs } - still.elapsed().as_secs_f64();
        if left <= 0.0 {
            break;
        }
        // commits held to be applied one an update (OneBatch): the next is applied without waiting
        if app.world().get_resource::<crate::OneBatch>().is_some_and(|q| !q.0.is_empty()) {
            continue;
        }
        seen = crate::wait_flush(seen, std::time::Duration::from_secs_f64(left.min(0.5)), ask.as_deref());
    }
    // a call past HANG_SECS is a hang, named: which runtime's call, and how long it has run
    if let Some(d) = crate::js_call_for().filter(|d| d.as_secs_f64() >= HANG_SECS) {
        return Err(format!("the composition hangs on frame {want:?}: {} has run for {:.0} s without returning (past {HANG_SECS} s: an endless loop, or a native call that never returns; `sample <pid>` shows where), its marker at frame {} (done {})", crate::js_call_who(), d.as_secs_f64(), marked.0, marked.1));
    }
    if held {
        return Err(format!("frame {want:?} was held for {HOLD_SECS} s and never released: a delayRender never continued, or a scene that never finished loading"));
    }
    Err(format!("the composition made no progress on frame {want:?} for {secs} s, its marker at frame {} (done {}) (does it throw? its [js] lines are above, and `raea traces show last` has the run's log)", marked.0, marked.1))
}

/// The entity carrying the frame marker (data-raea-frame), once found.
#[derive(Resource)]
pub struct MarkerEntity(Entity);
