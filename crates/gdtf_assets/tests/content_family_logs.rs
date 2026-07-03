//! GTW-570 content-family LOG-LINE tests — the fail-closed `warn!` (C2(b)) and
//! the hot-reload `info!` (C2(e), the GTW-374 Part C convention), asserted on
//! the real generic systems.
//!
//! Lives in its OWN test binary (the GTW-566 recipe): the capture layer is a
//! PROCESS-GLOBAL `tracing` default (a scoped `with_default` risks the
//! interest-cache poison), with per-thread buffer routing so only the
//! capturing thread records. Each assertion drives the system SYNCHRONOUSLY on
//! the calling thread via `run_system_once`, so its emission lands in this
//! thread's buffer deterministically. The harness disables `LogPlugin`, so
//! there is no competing global subscriber.

use std::{cell::RefCell, collections::HashMap, sync::OnceLock};

use bevy::{
    asset::{AssetEvent, Assets},
    ecs::system::RunSystemOnce,
    log::{
        tracing::{Event, Subscriber, callsite::rebuild_interest_cache, field::Visit},
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    prelude::*,
    reflect::TypePath,
};
use gdtf_assets::{
    ContentFamily, ContentFamilyAppExt, ContentFileStem, RonAsset, redrive_content_family,
    resolve_content_family,
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

thread_local! {
    /// The buffer the global [`CaptureLayer`] appends event messages to FOR THE
    /// CURRENT THREAD — `Some(..)` only while a [`capture_logs`] body runs here.
    static CAPTURE_BUFFER: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Installs the process-global capture subscriber exactly once, then rebuilds
/// the callsite interest cache so a pre-install emission can't leave a callsite
/// cached `never` (the GTW-494/GTW-566 determinism recipe).
static GLOBAL_CAPTURE: OnceLock<()> = OnceLock::new();

/// Install the global [`CaptureLayer`] if not already installed.
fn install_global_capture() {
    GLOBAL_CAPTURE.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        rebuild_interest_cache();
    });
}

/// A `tracing` [`Layer`] recording each event's `message` field into the
/// CURRENT THREAD's buffer (only while that thread is capturing).
struct CaptureLayer;

/// Pulls the `message` field's debug rendering out of an event.
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

/// Run `body` with this thread's capture buffer armed, returning every event
/// message recorded on this thread while it ran.
fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}

/// The stem-keyed test payload (`*.swatch.ron`) — mirrors `content_family.rs`.
#[derive(Deserialize, TypePath, Debug, Clone)]
struct Swatch {
    /// A distinguishable magnitude (unasserted here — the log is the subject).
    #[allow(dead_code, reason = "the log line, not the payload, is under test")]
    tone: u32,
}

/// The stem-keyed registry the swatch folder resolves into.
#[derive(Resource, Default, Debug)]
struct SwatchRegistry(HashMap<String, u32>);

/// The stem-keyed test family over the real fixture folder.
struct SwatchFamily;

impl ContentFamily for SwatchFamily {
    type Spec = Swatch;
    type Registry = SwatchRegistry;

    const EXTENSION: &'static str = "swatch.ron";
    const FOLDER: &'static str = "test/content_family";

    fn insert_member(registry: &mut SwatchRegistry, stem: Option<ContentFileStem>, spec: &Swatch) {
        let Some(stem) = stem else { return };
        registry.0.insert(stem.into_inner(), spec.tone);
    }
}

/// The PAYLOAD-KEYED test payload (`*.badge.ron`) sharing the mixed fixture
/// folder — its loader must register alongside the swatches' for the recursive
/// folder load to reach `Loaded` (an extension-less member fails the folder).
#[derive(Deserialize, TypePath, Debug, Clone)]
struct Badge {
    /// The payload-owned registry key.
    key:   String,
    /// Unread here — the log line, not the payload, is under test.
    #[allow(dead_code, reason = "the log line, not the payload, is under test")]
    glyph: String,
}

/// The payload-keyed registry the badge members resolve into.
#[derive(Resource, Default, Debug)]
struct BadgeRegistry(std::collections::HashSet<String>);

/// The payload-keyed companion family for the mixed folder.
struct BadgeFamily;

impl ContentFamily for BadgeFamily {
    type Spec = Badge;
    type Registry = BadgeRegistry;

    const EXTENSION: &'static str = "badge.ron";
    const FOLDER: &'static str = "test/content_family";

    fn insert_member(registry: &mut BadgeRegistry, _stem: Option<ContentFileStem>, spec: &Badge) {
        registry.0.insert(spec.key.clone());
    }
}

/// A payload for the FAILED-folder family (no member file exists).
#[derive(Deserialize, TypePath, Debug, Clone)]
struct Relic {
    /// Unread — no relic file exists.
    #[allow(
        dead_code,
        reason = "no member file exists; the field anchors the schema"
    )]
    age: u32,
}

/// The registry the missing relic folder fails closed into (EMPTY).
#[derive(Resource, Default, Debug)]
struct RelicRegistry(HashMap<String, u32>);

/// A family whose folder is deliberately MISSING.
struct RelicFamily;

impl ContentFamily for RelicFamily {
    type Spec = Relic;
    type Registry = RelicRegistry;

    const EXTENSION: &'static str = "relic.ron";
    const FOLDER: &'static str = "test/content_family_missing";

    fn insert_member(registry: &mut RelicRegistry, stem: Option<ContentFileStem>, spec: &Relic) {
        let Some(stem) = stem else { return };
        registry.0.insert(stem.into_inner(), spec.age);
    }
}

/// Generous SAFETY-NET cap on `App::update()` iterations while signal-polling
/// an async load — NOT a timing budget (the GTW-319 flake lesson).
const GENEROUS_LOAD_UPDATES: u32 = 10_000;

/// C2(b): the fail-closed resolve `warn!`s naming the folder and the registry
/// it emptied. Driven to the terminal `Failed` state through the real
/// ext-registered chain, then re-run synchronously (registry removed, so the
/// absence-gated failure path re-fires) on the capturing thread.
///
/// Pin-discriminating: removing the `warn!` (or the empty-registry insert)
/// leaves the capture empty / the registry absent.
#[test]
fn failed_folder_resolve_warns_naming_folder_and_registry() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.register_content_family::<RelicFamily>();
    // The registered chain reaches the terminal Failed state and fails closed.
    gdtf_test_utils::advance_until_resource_exists::<RelicRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    // Re-arm the failure path and re-run the REAL resolve system on THIS
    // thread so its warn! lands in the armed capture buffer.
    app.world_mut().remove_resource::<RelicRegistry>();
    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(resolve_content_family::<RelicFamily>);
        assert!(result.is_ok(), "the resolve system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("test/content_family_missing") && line.contains("RelicRegistry")
        }),
        "the fail-closed resolve must warn! naming the folder + the emptied registry; \
         captured: {captured:?}",
    );
    assert!(
        app.world().get_resource::<RelicRegistry>().is_some(),
        "the warned failure path must still insert the EMPTY registry",
    );
}

/// C2(e): the redrive logs the GTW-374 Part C `info!` line naming the rebuilt
/// registry type + folder. Run synchronously on the capturing thread against
/// the real redrive system.
///
/// Pin-discriminating: removing the `info!` leaves the capture empty.
#[test]
fn modified_member_redrive_logs_an_info_line() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    // BOTH families register (the folder is mixed — every member extension
    // needs a loader for the recursive folder load to reach `Loaded`).
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    // Edit a member in place and hand-write its Modified event, then run the
    // REAL redrive on this thread so the info! lands in the capture buffer.
    let alpha = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<Swatch>>("test/content_family/alpha.swatch.ron");
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<Swatch>>>()
        .get_mut(&alpha)
    {
        **asset = Swatch { tone: 11 };
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: alpha.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_content_family::<SwatchFamily>);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("hot-reload")
                && line.contains("SwatchRegistry")
                && line.contains("test/content_family")
        }),
        "the redrive must emit the Part C info! naming the registry + folder; \
         captured: {captured:?}",
    );
}
