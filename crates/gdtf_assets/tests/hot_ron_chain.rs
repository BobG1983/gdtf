use bevy::{
    asset::{AssetServer, Handle},
    prelude::*,
    reflect::TypePath,
};
use gdtf_assets::{HotRonAppExt, HotRonChain, HotRonHandle, RonAsset};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

#[derive(Resource, Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct HotSwatch {
        label: String,
        count: u32,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
struct MappedSwatch {
        doubled: u32,
            sub:     Handle<RonAsset<HotSwatch>>,
}

fn map_swatch(spec: &HotSwatch, asset_server: &AssetServer) -> MappedSwatch {
    MappedSwatch {
        doubled: spec.count * 2,
        sub:     asset_server.load::<RonAsset<HotSwatch>>(GOOD_PATH),
    }
}

fn fallback_swatch() -> HotSwatch {
    HotSwatch {
        label: "fallback".to_owned(),
        count: 0,
    }
}

const GOOD_PATH: &str = "test/hot_ron_fixture.ron";

const MALFORMED_PATH: &str = "test/hot_ron_malformed.ron";

const GENEROUS_LOAD_UPDATES: u32 = 10_000;

fn real_asset_app() -> App {
    GdtfUiTestAppBuilder::new().with_ui_camera().build()
}

#[test]
fn plain_chain_loads_and_resolves_exactly_once() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource::<HotSwatch>(GOOD_PATH);

    gdtf_test_utils::advance_until_resource_exists::<HotSwatch>(&mut app, GENEROUS_LOAD_UPDATES);

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&HotSwatch {
            label: "molten".to_owned(),
            count: 3,
        }),
        "the resolved resource must carry the on-disk fixture values",
    );
    assert!(
        app.world()
            .get_resource::<HotRonHandle<HotSwatch>>()
            .is_some(),
        "the kick-off must have stored the persistent generic handle",
    );

    let sentinel = HotSwatch {
        label: "sentinel".to_owned(),
        count: 99,
    };
    app.world_mut().insert_resource(sentinel.clone());
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&sentinel),
        "the resolve must insert exactly once — never re-publish over live data",
    );
}

#[test]
fn mapped_chain_derives_resource_with_asset_server_access() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource_mapped::<HotSwatch, MappedSwatch>(GOOD_PATH, map_swatch);

    gdtf_test_utils::advance_until_resource_exists::<MappedSwatch>(&mut app, GENEROUS_LOAD_UPDATES);

    let mapped = app.world().get_resource::<MappedSwatch>();
    assert_eq!(
        mapped.map(|m| m.doubled),
        Some(6),
        "the map hook must derive the resource from the payload (3 doubled)",
    );
    assert_ne!(
        mapped.map(|m| m.sub.clone()),
        Some(Handle::<RonAsset<HotSwatch>>::default()),
        "the map hook must have loaded its sub-asset THROUGH the server (non-default handle)",
    );
}

#[test]
fn failed_load_with_fallback_inserts_the_default() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource_with_fallback::<HotSwatch>(MALFORMED_PATH, fallback_swatch);

    gdtf_test_utils::advance_until_resource_exists::<HotSwatch>(&mut app, GENEROUS_LOAD_UPDATES);

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&fallback_swatch()),
        "a genuine Failed must fall back to the chain's default",
    );
}

#[test]
fn good_load_with_fallback_resolves_the_file_not_the_default() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource_with_fallback::<HotSwatch>(GOOD_PATH, fallback_swatch);

    gdtf_test_utils::advance_until_resource_exists::<HotSwatch>(&mut app, GENEROUS_LOAD_UPDATES);

    assert_eq!(
        app.world().get_resource::<HotSwatch>(),
        Some(&HotSwatch {
            label: "molten".to_owned(),
            count: 3,
        }),
        "with a fallback registered, a good load must still resolve the FILE values — \
         the default may fire only on a genuine Failed",
    );
}

#[test]
fn failed_load_without_fallback_leaves_the_resource_absent() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource::<HotSwatch>(MALFORMED_PATH);

    app.update();
    let id = app
        .world()
        .get_resource::<HotRonHandle<HotSwatch>>()
        .map(|handle| handle.id());
    assert!(id.is_some(), "the kick-off must have stored the handle");
    let Some(id) = id else { return };
    gdtf_test_utils::advance_until_load_state(
        &mut app,
        id,
        |state| state.is_failed(),
        GENEROUS_LOAD_UPDATES,
    );

    for _ in 0..4 {
        app.update();
    }
    assert!(
        app.world().get_resource::<HotSwatch>().is_none(),
        "without a fallback, a failed load must leave the resource absent",
    );
}

#[test]
fn headless_minimal_app_stays_a_no_op() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_hot_ron_resource::<HotSwatch>(GOOD_PATH);

    for _ in 0..3 {
        app.update();
    }

    assert!(
        app.world().get_resource::<HotSwatch>().is_none(),
        "a headless app must never resolve the resource",
    );
    assert!(
        app.world()
            .get_resource::<HotRonHandle<HotSwatch>>()
            .is_none(),
        "a headless app must never kick off the load",
    );
    assert!(
        app.world()
            .get_resource::<HotRonChain<HotSwatch, HotSwatch>>()
            .is_none(),
        "the self-gated registration must not even insert the chain config",
    );
}
