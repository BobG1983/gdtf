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

#[derive(Resource, Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct HotSwatch {
        label: String,
        count: u32,
}

#[derive(Resource, Default)]
struct RederiveSeen(u32);

const CHAIN_PATH: &str = "test/hot_ron_fixture.ron";

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default());
    app.init_hot_ron_resource::<HotSwatch>(CHAIN_PATH);
    app
}

fn swatch(label: &str) -> HotSwatch {
    HotSwatch {
        label: label.to_owned(),
        count: 1,
    }
}

fn add_asset(app: &mut App, value: HotSwatch) -> Handle<RonAsset<HotSwatch>> {
    app.world_mut()
        .resource_mut::<Assets<RonAsset<HotSwatch>>>()
        .add(RonAsset::new(value))
}

fn hot_edit(app: &mut App, handle: &Handle<RonAsset<HotSwatch>>, value: HotSwatch) {
    let mut assets = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<HotSwatch>>>();
    if let Some(mut asset) = assets.get_mut(handle) {
        **asset = value;
    }
}

fn inject_modified(app: &mut App, handle: &Handle<RonAsset<HotSwatch>>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

fn stage_resolved(app: &mut App, baseline: HotSwatch) -> Handle<RonAsset<HotSwatch>> {
    app.update();
    let handle = add_asset(app, baseline.clone());
    app.world_mut().insert_resource(baseline);
    app.world_mut()
        .insert_resource(HotRonHandle::new(handle.clone()));
    handle
}

#[test]
fn modified_rederives_in_place_and_marks_changed() {
    let mut app = app();
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

    app.update();
    app.update();
    let before = app.world().resource::<RederiveSeen>().0;

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

#[test]
fn modified_for_other_id_does_not_rederive() {
    let mut app = app();
    let baseline = swatch("baseline");
    let _active = stage_resolved(&mut app, baseline.clone());
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

#[test]
fn pre_resolve_events_are_drained_not_replayed() {
    let mut app = app();
    app.update();

    let pending = add_asset(&mut app, swatch("pending"));
    inject_modified(&mut app, &pending);
    app.update();

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

fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    install_global_capture();
    let prior = CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().replace(Vec::new()));
    body();
    let captured =
        CAPTURE_BUFFER.with(|buffer| std::mem::replace(&mut *buffer.borrow_mut(), prior));
    captured.unwrap_or_default()
}

#[test]
fn redrive_logs_the_concrete_type_and_path() {
    let mut app = app();
    let handle = stage_resolved(&mut app, swatch("baseline"));
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
