//! GTW-374 hot-reload TEST support — a tiny `tracing` capture layer the
//! `tuning` / `weapons` / `armor` redrive tests use to prove their `info!`
//! hot-reload line actually fires on the real code path (Part C: every hot-reload
//! path logs naming what reloaded).
//!
//! Bevy 0.19 re-exports `tracing` + `tracing-subscriber` through [`bevy::log`], so a
//! test can install a scoped [`CaptureLayer`] via
//! [`bevy::log::tracing::subscriber::with_default`] and assert the captured event
//! messages contain the expected hot-reload line — no extra workspace dependency.

use std::sync::{Arc, Mutex};

use bevy::log::{
    tracing::{
        Event, Subscriber, callsite::rebuild_interest_cache, field::Visit, subscriber::with_default,
    },
    tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
};

/// A shared buffer of captured log-event MESSAGES (the `message` field of each
/// `info!` / `warn!` / … emitted while the [`CaptureLayer`] is the active subscriber).
type CapturedMessages = Arc<Mutex<Vec<String>>>;

/// A `tracing` [`Layer`] that records each event's rendered `message` field into a
/// shared [`CapturedMessages`] buffer — the minimal capture a test needs to assert a
/// specific `info!` line fired.
struct CaptureLayer {
    /// The shared buffer the captured messages are appended to.
    messages: CapturedMessages,
}

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
        if let Some(message) = visitor.message
            && let Ok(mut buffer) = self.messages.lock()
        {
            buffer.push(message);
        }
    }
}

/// Run `body` with a scoped [`CaptureLayer`] installed as the active `tracing`
/// subscriber, returning every event MESSAGE captured while it ran.
///
/// The subscriber is scoped to this call (`with_default`), so it never leaks into
/// other tests. The returned `Vec` is in emission order; a test asserts it CONTAINS
/// the expected hot-reload line.
///
/// GTW-455: rebuild the `tracing` callsite-interest cache from INSIDE the
/// `with_default` scope before running `body`. `tracing-core` caches per-callsite
/// `Interest` GLOBALLY (shared across threads). `with_default(subscriber, f)` builds
/// the `Dispatch` (which rebuilds interest) BEFORE it installs the subscriber as the
/// thread-local default, so when `tracing`'s `has_just_one` fast path is active that
/// rebuild evaluates a hot-reload `info!` callsite against the still-current
/// `NoSubscriber` and can cache it `Interest::never()` — globally. A FIRST emission of
/// that callsite (cold start, e.g. the first multi-threaded run after a rebuild) then
/// short-circuits and the capture comes back EMPTY (the GTW-455 flake). Forcing a
/// rebuild here — now that the capture subscriber IS the thread-local default —
/// re-evaluates the callsite against the live capture subscriber, so the emission is
/// never spuriously elided. Deterministic, no sleeps/retries; fixes every
/// `*_hot_reload_logs_an_info_line` test that shares this helper.
pub(in crate::states::load) fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    let messages: CapturedMessages = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer {
        messages: Arc::clone(&messages),
    };
    let subscriber = Registry::default().with(layer);
    with_default(subscriber, || {
        rebuild_interest_cache();
        body();
    });
    let captured = messages.lock().map(|buffer| buffer.clone());
    captured.unwrap_or_default()
}
