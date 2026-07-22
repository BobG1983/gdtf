//! Integration tests for the GTW-564 hot-RON machinery's REDRIVE half — the live
//! `Modified` re-derive, its four encoded traps, and its reload log line.
//!
//! Each app registers the WHOLE chain through the real
//! [`HotRonAppExt`](gdtf_assets::HotRonAppExt) ext call (no stubs, no shadow
//! copies); the file-watcher is stood in for by the established house pattern —
//! an in-memory `Assets::get_mut` edit plus an injected
//! [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) message.

use std::{cell::RefCell, sync::OnceLock};

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, Assets, Handle},
    ecs::system::RunSystemOnce,
    log::{
        tracing::{
            Event, Subscriber,
            callsite::rebuild_interest_cache,
            field::{Field, Visit},
        },
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    prelude::*,
    reflect::TypePath,
};
use gdtf_assets::{HotRonAppExt, HotRonHandle, RonAsset, redrive_hot_ron_resource};
use serde::Deserialize;

/// The test payload/resource of the redrive suite — the plain-variant shape
/// (payload IS the resource), mirroring `hot_ron_chain.rs`'s `HotSwatch`.
#[derive(Resource, Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct HotSwatch {
    /// The distinguishing field the re-derive asserts on.
    label: String,
    /// A second field so edits are structurally realistic.
    count: u32,
}

/// Counts how many times the change-detection probe observed `HotSwatch`
/// CHANGED — the witness that the redrive's `ResMut` overwrite drives Bevy
/// change detection (trap (d)).
#[derive(Resource, Default)]
struct RederiveSeen(u32);

/// The chain path the ext call registers. The file does not exist under the
/// test-crate asset root — irrelevant here, because every test re-points the
/// generic handle at a hand-inserted in-memory asset (the house redrive-test
/// pattern).
const CHAIN_PATH: &str = "test/hot_ron_fixture.ron";

/// A headless app with the REAL ext-registered chain: `MinimalPlugins` +
/// `AssetPlugin` (a real `AssetServer`, the `Assets` collection, and the
/// `AssetEvent` message buffer), then the one
/// [`init_hot_ron_resource`](gdtf_assets::HotRonAppExt::init_hot_ron_resource)
/// call — kick-off, resolve, and redrive all registered by the machinery itself.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default());
    app.init_hot_ron_resource::<HotSwatch>(CHAIN_PATH);
    app
}

/// A `HotSwatch` with the given label (count fixed) — the distinct fixtures the
/// re-derive tests contrast.
fn swatch(label: &str) -> HotSwatch {
    HotSwatch {
        label: label.to_owned(),
        count: 1,
    }
}

/// Add a `RonAsset<HotSwatch>` to the collection and return its handle.
fn add_asset(app: &mut App, value: HotSwatch) -> Handle<RonAsset<HotSwatch>> {
    app.world_mut()
        .resource_mut::<Assets<RonAsset<HotSwatch>>>()
        .add(RonAsset::new(value))
}

/// Overwrite the in-memory payload of an already-added asset (the hot edit the
/// file-watcher would make on a re-save).
fn hot_edit(app: &mut App, handle: &Handle<RonAsset<HotSwatch>>, value: HotSwatch) {
    let mut assets = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<HotSwatch>>>();
    if let Some(mut asset) = assets.get_mut(handle) {
        **asset = value;
    }
}

/// Inject an `AssetEvent::Modified` for the given handle id (standing in for
/// the file-watcher's reload signal).
fn inject_modified(app: &mut App, handle: &Handle<RonAsset<HotSwatch>>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

/// Stage a resolved chain: a hand-inserted baseline asset, the generic handle
/// re-pointed at it, and the baseline resource resident.
fn stage_resolved(app: &mut App, baseline: HotSwatch) -> Handle<RonAsset<HotSwatch>> {
    // Run Startup so the ext-registered kick-off has run (its handle is then
    // overwritten with the hand-minted one — resources overwrite in place).
    app.update();
    let handle = add_asset(app, baseline.clone());
    app.world_mut().insert_resource(baseline);
    app.world_mut()
        .insert_resource(HotRonHandle::new(handle.clone()));
    handle
}

/// C8: a `Modified` for the active handle re-derives the resident resource IN
/// PLACE from the updated in-memory value, and the `ResMut` overwrite is
/// OBSERVED by Bevy change detection (trap (d)) — a `resource_changed`-gated
/// probe fires.
///
/// Pin-discriminating: dropping the re-derive leaves the OLD label; replacing
/// the `ResMut` overwrite with a remove+insert or a silent write would still
/// mark changed here, but dropping the write entirely leaves the probe count
/// flat.
#[test]
fn modified_rederives_in_place_and_marks_changed() {
    let mut app = app();
    // The change-detection witness, ordered after the redrive so it observes
    // the same frame's overwrite deterministically (bevy-traps #3).
    app.init_resource::<RederiveSeen>();
    app.add_systems(
        Update,
        (|mut seen: ResMut<RederiveSeen>| {
            seen.0 += 1;
        })
        .after(redrive_hot_ron_resource::<HotSwatch, HotSwatch>)
        .run_if(resource_exists::<HotSwatch>.and_then(resource_changed::<HotSwatch>)),
    );

    let baseline = swatch("baseline");
    let handle = stage_resolved(&mut app, baseline);

    // Settle: the insert itself counts as one change; steady frames add none.
    app.update();
    app.update();
    let before = app.world().resource::<RederiveSeen>().0;

    // Hot-edit to a DISTINCT value, then fire a Modified for the active id.
    let edited = swatch("edited");
    hot_edit(&mut app, &handle, edited.clone());
    inject_modified(&mut app, &handle);
    app.update();

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&edited),
        "a Modified for the active handle must re-derive the resource to the edited value",
    );
    assert!(
        app.world().resource::<RederiveSeen>().0 > before,
        "the ResMut overwrite must mark the resource CHANGED so downstream re-derives run",
    );
}

/// C8 / trap (c): a `Modified` for a DIFFERENT asset id leaves the resource
/// untouched — the filter is on the ACTIVE handle id only.
#[test]
fn modified_for_other_id_does_not_rederive() {
    let mut app = app();
    let baseline = swatch("baseline");
    let _active = stage_resolved(&mut app, baseline.clone());
    // A second, unrelated asset whose value differs from the active one.
    let other = add_asset(&mut app, swatch("other"));

    app.update();
    hot_edit(&mut app, &other, swatch("other-edited"));
    inject_modified(&mut app, &other);
    app.update();

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&baseline),
        "a Modified for a non-active id must NOT re-derive the resource",
    );
}

/// C8 / trap (b): an event arriving BEFORE the chain resolves is DRAINED, not
/// left to linger — once the resources arrive, the stale event must not
/// re-fire and clobber the freshly-staged value.
///
/// Pin-discriminating: a redrive that early-returned WITHOUT `events.clear()`
/// would still see the pre-resolve event in its reader on the next frame and
/// re-derive from it.
#[test]
fn pre_resolve_events_are_drained_not_replayed() {
    let mut app = app();
    // Startup: kick-off ran; the resource is still ABSENT (nothing to resolve —
    // the chain path has no file under the test-crate asset root).
    app.update();

    // A pre-resolve Modified for an asset the handle does NOT yet point at:
    // with the resource absent the redrive must DRAIN it.
    let pending = add_asset(&mut app, swatch("pending"));
    inject_modified(&mut app, &pending);
    app.update();

    // NOW the chain "resolves": baseline resource + handle pointing at the
    // pending asset. If the stale event lingered, the next update would
    // re-derive baseline -> "pending".
    let baseline = swatch("baseline");
    app.world_mut().insert_resource(baseline.clone());
    app.world_mut()
        .insert_resource(HotRonHandle::new(pending.clone()));
    app.update();

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&baseline),
        "a pre-resolve Modified must be drained, never replayed once the chain resolves",
    );
}

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
/// process-global per-callsite `Interest` cache under parallel test threads — a
/// concurrent thread's first-time emission can rebuild the cache while no
/// always-interested dispatcher is live and poison the captured callsite
/// `never`, so the `info!` short-circuits and the capture comes back empty. One
/// global always-interested default (plus a post-install
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

/// A `tracing` layer recording each event's `message` field into the CURRENT
/// THREAD's [`CAPTURE_BUFFER`] — the minimal capture proving the reload `info!`
/// fired (the ONE copy of the scaffold the per-site chains each duplicated;
/// theirs collapsed onto this suite). Mirrors the GTW-494 shared recipe in
/// `gdtf_app`'s `hot_reload_test_support` (not reachable from this crate).
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
/// The capture is per-thread, so concurrent tests never see each other's
/// events; the system must run SYNCHRONOUSLY via `run_system_once` on the
/// calling thread (the GTW-374 capture lesson) so its `info!` lands here.
/// Determinism rationale: see [`GLOBAL_CAPTURE`] (GTW-455).
fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}

/// C4: the hot-reload `info!` fires on the real re-derive path, naming the
/// CONCRETE resource type AND the chain path (the GTW-374 Part C convention).
///
/// Pin-discriminating: removing the `info!` (or dropping the type / path from
/// it) leaves the capture without a matching line.
#[test]
fn redrive_logs_the_concrete_type_and_path() {
    let mut app = app();
    let handle = stage_resolved(&mut app, swatch("baseline"));
    // Stage the hot edit + the Modified, then run the redrive synchronously
    // inside the capture scope (the schedule executor may use worker threads
    // the thread-local capture would miss).
    hot_edit(&mut app, &handle, swatch("edited"));
    inject_modified(&mut app, &handle);

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_hot_ron_resource::<HotSwatch, HotSwatch>);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| line.contains("hot-reload")
            && line.contains("HotSwatch")
            && line.contains(CHAIN_PATH)),
        "the redrive must emit an info! naming the concrete type + path; captured: {captured:?}",
    );
}
