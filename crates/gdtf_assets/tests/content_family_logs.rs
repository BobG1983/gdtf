//! Content-family log lines: fail-closed resolve warns; redrive logs info.
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
            static CAPTURE_BUFFER: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

static GLOBAL_CAPTURE: OnceLock<()> = OnceLock::new();

fn install_global_capture() {
    GLOBAL_CAPTURE.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        rebuild_interest_cache();
    });
}

struct CaptureLayer;

struct MessageVisitor {
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

fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}

#[derive(Deserialize, TypePath, Debug, Clone)]
struct Swatch {
        #[allow(dead_code, reason = "the log line, not the payload, is under test")]
    tone: u32,
}

#[derive(Resource, Default, Debug)]
struct SwatchRegistry(HashMap<String, u32>);

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

#[derive(Deserialize, TypePath, Debug, Clone)]
struct Badge {
        key:   String,
        #[allow(dead_code, reason = "the log line, not the payload, is under test")]
    glyph: String,
}

#[derive(Resource, Default, Debug)]
struct BadgeRegistry(std::collections::HashSet<String>);

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

#[derive(Deserialize, TypePath, Debug, Clone)]
struct Relic {
        #[allow(
        dead_code,
        reason = "no member file exists; the field anchors the schema"
    )]
    age: u32,
}

#[derive(Resource, Default, Debug)]
struct RelicRegistry(HashMap<String, u32>);

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

const GENEROUS_LOAD_UPDATES: u32 = 10_000;

#[test]
fn failed_folder_resolve_warns_naming_folder_and_registry() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.register_content_family::<RelicFamily>();
    gdtf_test_utils::advance_until_resource_exists::<RelicRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

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

#[test]
fn modified_member_redrive_logs_an_info_line() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

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
