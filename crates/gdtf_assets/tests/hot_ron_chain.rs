//! Integration tests for the GTW-564 hot-RON seam's LOAD half — the
//! ext-registered kick-off + resolve chain on a REAL `AssetServer` + the real
//! on-disk fixtures (`assets/test/hot_ron_*.ron`).
//!
//! The `HotSwatch` payload below IS the contract's "hypothetical 13th hot
//! resource" demonstration (P1): one payload type + one
//! [`HotRonAppExt`](gdtf_assets::HotRonAppExt) call (+ an optional map fn for
//! the mapped variant) is ALL a new chain needs — no per-site handle newtype,
//! no hand-rolled kick-off / resolve / redrive systems.

use bevy::{
    asset::{AssetServer, Handle},
    prelude::*,
    reflect::TypePath,
};
use gdtf_assets::{HotRonAppExt, HotRonChain, HotRonHandle, RonAsset};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

/// The "13th hot resource" payload used only by these tests: `Deserialize` (so
/// the RON loader parses it), `Resource` + `Clone` (so the PLAIN chain variant
/// can clone it in as the runtime resource), and `TypePath`/`Send`/`Sync`
/// (the `RonAsset<T>` payload bounds).
#[derive(Resource, Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct HotSwatch {
    /// A string field, to prove non-numeric fields resolve through the chain.
    label: String,
    /// A numeric field the mapped-variant test derives from.
    count: u32,
}

/// The MAPPED-variant runtime resource derived from [`HotSwatch`] — a different
/// type from the payload, carrying a sub-asset handle the map hook loads
/// THROUGH the [`AssetServer`] (the theme-font precedent).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
struct MappedSwatch {
    /// `HotSwatch::count` doubled — proves the map hook ran on the payload.
    doubled: u32,
    /// A sub-asset handle loaded through the server INSIDE the map hook —
    /// proves the hook genuinely has `AssetServer` access.
    sub:     Handle<RonAsset<HotSwatch>>,
}

/// The map hook of the mapped-variant test: derives [`MappedSwatch`] from the
/// payload AND loads a sub-asset through the server (the reason the hook is
/// `map(&Spec, &AssetServer) -> T`, not a pure `fn`).
fn map_swatch(spec: &HotSwatch, asset_server: &AssetServer) -> MappedSwatch {
    MappedSwatch {
        doubled: spec.count * 2,
        sub:     asset_server.load::<RonAsset<HotSwatch>>(GOOD_PATH),
    }
}

/// The `Failed -> default` fallback of the fallback tests.
fn fallback_swatch() -> HotSwatch {
    HotSwatch {
        label: "fallback".to_owned(),
        count: 0,
    }
}

/// Loose-file path (relative to the asset source root) of the well-formed
/// hot-RON fixture (`label: "molten", count: 3`).
const GOOD_PATH: &str = "test/hot_ron_fixture.ron";

/// Loose-file path of the deliberately malformed fixture (`count` is a string).
const MALFORMED_PATH: &str = "test/hot_ron_malformed.ron";

/// Generous SAFETY-NET cap on `App::update()` iterations while signal-polling
/// an async load — NOT a timing budget (the GTW-319 flake lesson).
const GENEROUS_LOAD_UPDATES: u32 = 10_000;

/// A real-`AssetServer` app (`DefaultPlugins`, backends: None, asset source
/// rooted at the workspace `assets/`) — the same harness the RON-loader tests
/// drive.
fn real_asset_app() -> App {
    GdtfUiTestAppBuilder::new().with_ui_camera().build()
}

/// C8: `load -> resolve` inserts the runtime resource EXACTLY ONCE through the
/// real ext-registered chain — the kick-off stores the generic
/// [`HotRonHandle`], the gated resolve publishes the on-disk values, and once
/// resolved it NEVER re-publishes over live data.
///
/// Pin-discriminating: an unregistered chain never resolves (the signal wait
/// times out); a resolve that kept running would clobber the post-resolve
/// sentinel back to the file values.
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

    // EXACTLY ONCE: overwrite the live resource with a sentinel; the resolve is
    // gated on the resource's ABSENCE, so further updates must not re-publish
    // the file values over it.
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

/// C8 / C1: the MAPPED variant derives a DIFFERENT resource type from the
/// payload, and its map hook runs WITH `AssetServer` access (it loads a
/// sub-asset — the `GdtfThemeSpec::resolve` font precedent).
///
/// Pin-discriminating: a pure-`fn` map could not have produced a non-default
/// sub-asset handle; a dropped map would leave `MappedSwatch` absent.
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

/// C8 / C4: with the fallback opted in, a GENUINE `LoadState::Failed` (the
/// malformed fixture) inserts the fallback default — the chain still publishes
/// a resource, so a presence-gated flow is never stranded.
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

/// C8 / C4: the fallback fires ONLY on a genuine `Failed` — a well-formed file
/// resolves to the ON-DISK values, never the default (a fallback that fired
/// while the load was still in flight would have published the default first
/// and, resolve being insert-once, the default would have stuck).
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

/// C8: WITHOUT a fallback, a failed load leaves the resource absent — exactly
/// the per-site behavior of the presenter role/tuning chains this seam
/// replaced (consumers stay gated on the resource's presence).
#[test]
fn failed_load_without_fallback_leaves_the_resource_absent() {
    let mut app = real_asset_app();
    app.init_hot_ron_resource::<HotSwatch>(MALFORMED_PATH);

    // Run Startup so the kick-off stores the handle, then signal-poll the load
    // to its terminal Failed state.
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

/// C8 / C3(a): a no-`AssetServer` `MinimalPlugins` app updates without panic —
/// registration self-gates, so the ext call registers NOTHING (no loader, no
/// chain config, no systems).
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
