//! The process-global `tracing` capture the redrive log pin rides — split from
//! `redrive.rs` at its natural concern boundary (module-layout warn band): this file
//! changes when the CAPTURE recipe does, `redrive.rs` when the redrive behavior
//! does.

use std::{cell::RefCell, sync::OnceLock};

use bevy::log::{
    tracing::{
        Event, Subscriber,
        callsite::rebuild_interest_cache,
        field::{Field, Visit},
    },
    tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
};

thread_local! {
    /// The buffer the process-global [`CaptureLayer`] appends captured event
    /// messages to FOR THE CURRENT THREAD — `Some(..)` only while a
    /// [`capture_logs`] body runs here, so a concurrent non-capturing test's
    /// events are dropped rather than bleeding into this capture.
    static CAPTURE_BUFFER: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Ensures the process-global [`CaptureLayer`] subscriber is installed EXACTLY
/// ONCE for this test binary (the GTW-494 determinism recipe, applied here by
/// GTW-455): a scoped `with_default` capture races `tracing-core`'s
/// process-global per-callsite `Interest` cache under parallel test threads —
/// a concurrent thread's first-time emission can rebuild the cache while no
/// always-interested dispatcher is live and poison the captured callsite
/// `never`, so the `info!` short-circuits and the capture comes back empty.
/// One global always-interested default (plus a post-install
/// [`rebuild_interest_cache`] to heal callsites registered while the default
/// was still `NoSubscriber`) closes both windows — no lock, sleep, or retry.
static GLOBAL_CAPTURE: OnceLock<()> = OnceLock::new();

/// Install the process-global [`CaptureLayer`] if not already installed, then
/// re-evaluate cached callsite interest against it (see [`GLOBAL_CAPTURE`]).
fn install_global_capture() {
    GLOBAL_CAPTURE.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        // First (and only) global default for this test process — a second call
        // would `Err`, which the `OnceLock` already prevents; no competing global
        // is installed in this binary (the harness adds no `LogPlugin`).
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        rebuild_interest_cache();
    });
}

/// A `tracing` layer that records each event's `message` field into the CURRENT
/// THREAD's [`CAPTURE_BUFFER`] — the minimal capture needed to prove the `info!`
/// hot-reload line fired. Mirrors the GTW-494 shared recipe in `gdtf_app`'s
/// `hot_reload_test_support` (no shared util is reachable here).
struct CaptureLayer;

/// Pulls the `message` field's debug rendering out of a `tracing` event.
struct MessageVisitor {
    /// The captured message text, if a `message` field was visited.
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
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

/// Run `body` with this thread's [`CAPTURE_BUFFER`] armed, returning every event
/// message the process-global [`CaptureLayer`] captured on this thread while it
/// ran (in emission order).
///
/// The capture is per-thread, so concurrent tests never see each other's events;
/// the observed system must run SYNCHRONOUSLY on this thread (the call site uses
/// `run_system_once`) so its `info!` lands here. Determinism rationale: see
/// [`GLOBAL_CAPTURE`] (GTW-455).
pub(super) fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}
