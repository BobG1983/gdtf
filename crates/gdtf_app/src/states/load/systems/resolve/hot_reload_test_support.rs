//! GTW-374 hot-reload TEST support — a tiny `tracing` capture layer the
//! `tuning` / `weapons` / `armor` redrive tests use to prove their `info!`
//! hot-reload line actually fires on the real code path (Part C: every hot-reload
//! path logs naming what reloaded).
//!
//! Bevy 0.19 re-exports `tracing` + `tracing-subscriber` through [`bevy::log`], so a
//! test can install a [`CaptureLayer`] subscriber and assert the captured event
//! messages contain the expected hot-reload line — no extra workspace dependency.

use std::{cell::RefCell, sync::OnceLock};

use bevy::log::{
    tracing::{Event, Subscriber, callsite::rebuild_interest_cache, field::Visit},
    tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
};

thread_local! {
    /// The buffer the [active][`CaptureLayer`] global subscriber appends captured
    /// event messages to FOR THE CURRENT THREAD — `Some(..)` only while a
    /// [`capture_logs`] body is running on this thread, `None` otherwise.
    ///
    /// Per-thread routing is the crux of the GTW-494 determinism fix (see
    /// [`capture_logs`]): the capture subscriber is the ONE process-global default, so
    /// events from EVERY thread reach its `on_event`, but only the thread that is
    /// actively capturing has a `Some` buffer — every other thread's events are
    /// dropped. So a concurrent non-capturing test's `info!`/`warn!` never lands in a
    /// capturing test's buffer.
    static CAPTURE_BUFFER: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Ensures the process-global [`CaptureLayer`] subscriber is installed EXACTLY ONCE for the
/// whole `gdtf_app` test binary — the GTW-494 determinism fix.
///
/// `tracing-core`'s per-callsite `Interest` is a SINGLE process-global cache
/// (`tracing-core-0.1.36` `callsite.rs`). With the previous scoped-`with_default` capture,
/// every `capture_logs` call registered a NEW scoped `Dispatch`, which flips
/// `Dispatchers::has_just_one` to `false` permanently; thereafter ANY thread that registers a
/// new callsite (e.g. a concurrent non-capture test emitting its first `info!`) re-evaluates
/// our shared hot-reload callsite by AND-ing the interest of EVERY currently-live scoped
/// `Dispatch`. In the micro-window where our prior capture `Dispatch` has died (pruned from
/// the set) and the next has not yet registered, that AND-set contains only OTHER threads'
/// `never`-returning dispatchers, so the callsite gets cached `Interest::never()` and the next
/// `info!` short-circuits — the capture comes back EMPTY (the residual flake GTW-487/494
/// surfaced by adding more concurrent capture tests).
///
/// Installing ONE process-global default (`set_global_default`) and NEVER creating a scoped
/// `Dispatch` keeps `has_just_one == true` forever, so the interest rebuild always evaluates
/// against our single, always-interested global subscriber (a bare `Registry` + `CaptureLayer`
/// is `Interest::always`) and can never be poisoned `never` by another thread. Deterministic,
/// no lock / sleeps / retries. Cross-test BINARIES are separate processes with separate caches,
/// so installing once per process is correct and sufficient. The `OnceLock` makes the install
/// idempotent + thread-safe; a second `set_global_default` would error, which the guard avoids.
///
/// The INSTALL-ORDERING hole that survived the first global-default attempt: the default global
/// dispatch is `NoSubscriber` UNTIL [`install_global_capture`] runs, and `NoSubscriber::`
/// `register_callsite` returns `Interest::never()` (`tracing-core-0.1.36` `subscriber.rs`). A
/// NON-capture test that emits a hot-reload `info!` directly (e.g. the
/// `modified_member_rebuilds_*` tests drive the redrive via `app.update()` with NO
/// `capture_logs`) REGISTERS that callsite while the global is still `NoSubscriber`, so its
/// interest is cached `never` PROCESS-GLOBALLY — and a later `capture_logs` test that fires the
/// same callsite then short-circuits and captures `[]`. [`install_global_capture`] closes this
/// by calling [`rebuild_interest_cache`] right after the global becomes the default, re-evaluating
/// every already-registered callsite against the now-live always-interested global (the rebuilder
/// is `JustOne` → `get_default`), so any callsite poisoned `never` by a pre-install emission is
/// reset to `always`. Once installed, `has_just_one == true` means every subsequent registration
/// consults the global directly, so no callsite can be re-poisoned thereafter.
static GLOBAL_CAPTURE: OnceLock<()> = OnceLock::new();

/// Install the process-global [`CaptureLayer`] subscriber if it is not already installed, then
/// re-evaluate cached callsite interest against it (closing the pre-install `NoSubscriber`
/// poison window — see [`GLOBAL_CAPTURE`]).
fn install_global_capture() {
    GLOBAL_CAPTURE.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        // First (and only) global default for this test process — ignore the result: a second
        // call would `Err` (already set), which the `OnceLock` already prevents, and a foreign
        // global default (none is installed in the `gdtf_app` lib test binary) would only mean
        // we capture nothing, which the assertions would catch loudly rather than silently.
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        // Now that our always-interested subscriber IS the global default, re-evaluate every
        // callsite already registered against the prior `NoSubscriber` (which cached them
        // `never`), so a hot-reload `info!` emitted by a non-capture test before this install
        // is no longer short-circuited when a capture test later fires it.
        rebuild_interest_cache();
    });
}

/// A `tracing` [`Layer`] that records each event's rendered `message` field into the
/// CURRENT THREAD's [`CAPTURE_BUFFER`] — but ONLY when that thread is actively capturing
/// (its buffer is `Some`). Installed once as the process-global default; per-thread routing
/// keeps a capturing test's buffer free of other threads' events.
struct CaptureLayer;

/// A field [`Visit`]or that pulls out the `message` field's debug rendering — the
/// text an `info!("…")` macro records under the reserved `message` field.
struct MessageVisitor {
    /// The captured message text, if a `message` field was visited.
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(
        &mut self,
        field: &bevy::log::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor { message: None };
        event.record(&mut visitor);
        let Some(message) = visitor.message else {
            return;
        };
        CAPTURE_BUFFER.with(|buffer| {
            if let Some(messages) = buffer.borrow_mut().as_mut() {
                messages.push(message);
            }
        });
    }
}

/// Run `body` with this thread's [`CAPTURE_BUFFER`] armed, returning every event MESSAGE the
/// process-global [`CaptureLayer`] captured on this thread while it ran.
///
/// The returned `Vec` is in emission order; a test asserts it CONTAINS the expected hot-reload
/// line. The capture is per-thread (the buffer is a `thread_local!`) so concurrent capturing
/// tests never see each other's events, and the redrive must be driven SYNCHRONOUSLY on this
/// thread (the call sites use `World::run_system_once`) so its `info!` is recorded here.
///
/// GTW-494 determinism: see [`install_global_capture`] — the capture subscriber is the ONE
/// process-global default and no scoped `Dispatch` is ever created, so `tracing`'s global
/// per-callsite `Interest` cache can never be poisoned `never` by a concurrent thread; and the
/// install re-evaluates already-registered callsites so a pre-install `NoSubscriber` emission
/// can't leave one cached `never` either. Both halves close the empty-capture flake without any
/// lock, sleep, or retry. The assertion is UNCHANGED — it still proves the real `info!` fired.
pub(crate) fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    // Arm this thread's buffer, capturing any value it already held (always `None` in practice,
    // since `capture_logs` calls are not nested) to restore on the way out.
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    // Take the captured messages back out and restore the prior buffer state.
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}
