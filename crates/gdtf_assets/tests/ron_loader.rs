//! Integration tests for the generic RON [`AssetLoader`](gdtf_assets) on a real
//! `AssetServer`.
//!
//! These exercise the actual asset pipeline — not a stub — through the headless
//! `GdtfUiTestAppBuilder` harness (`DefaultPlugins` with `backends: None`, which
//! brings up the `AssetPlugin`/`AssetServer` and points the file source at the
//! workspace-root `assets/` dir). A well-formed loose `.ron` is loaded, driven
//! to completion, and its deserialized fields asserted; a malformed `.ron` is
//! loaded and asserted to fail with a typed `LoadState::Failed` (no panic).

use bevy::{
    asset::{AssetServer, Assets, Handle},
    reflect::TypePath,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

/// A tiny payload type used only by these tests.
///
/// It is `serde`-`Deserialize` (so the RON loader can parse it) and is
/// `TypePath`, `Send`, `Sync`, and `'static` (the bounds `RonAsset<T>` requires
/// of any payload). The fixture `assets/test/ron_loader_fixture.ron`
/// deserializes into it.
#[derive(Deserialize, TypePath, Debug, PartialEq, Eq)]
struct LoaderFixture {
    /// A string field, to prove non-numeric fields round-trip.
    label: String,
    /// A numeric field, to prove typed fields round-trip — and, in the
    /// malformed fixture, to force a deserialization failure.
    count: u32,
}

/// Loose-file path (relative to the asset source root) of the well-formed
/// fixture.
const GOOD_FIXTURE_PATH: &str = "test/ron_loader_fixture.ron";

/// Loose-file path of the deliberately malformed fixture (`count` is a string).
const MALFORMED_FIXTURE_PATH: &str = "test/ron_loader_malformed.ron";

/// Max `App::update()` iterations to wait for an async asset load to settle.
/// Generous so a slow CI box never flakes; the loads settle in a few frames.
const MAX_LOAD_UPDATES: u32 = 64;

/// The generic RON loader resolves a well-formed loose `.ron` into a typed
/// `RonAsset<LoaderFixture>` whose fields equal the on-disk values.
///
/// Pin-discriminating: this drives the **real** `AssetServer` + the registered
/// `RonAssetLoader<LoaderFixture>`. If the loader were not registered (the
/// `init_ron_asset` clause), the asset would never resolve and the wait would
/// time out (`assert!(loaded, ...)` fails). If the loader deserialized into the
/// wrong shape, the field asserts fail. If the source root were wrong (the
/// repo-root `assets/` clause), the file would not be found and the load would
/// fail rather than resolve.
#[test]
fn well_formed_ron_resolves_to_typed_asset() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.init_ron_asset::<LoaderFixture>();

    let handle: Handle<RonAsset<LoaderFixture>> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load(GOOD_FIXTURE_PATH)
    };

    // Drive the app until the asset collection contains the resolved asset.
    let id = handle.id();
    let loaded = gdtf_test_utils::advance_until(
        &mut app,
        |app| {
            app.world()
                .resource::<Assets<RonAsset<LoaderFixture>>>()
                .get(id)
                .is_some()
        },
        MAX_LOAD_UPDATES,
    );

    assert!(
        loaded,
        "RonAsset<LoaderFixture> should resolve from {GOOD_FIXTURE_PATH} within \
         {MAX_LOAD_UPDATES} updates — proves the loader is registered and the \
         asset source root resolves the loose fixture",
    );

    let assets = app.world().resource::<Assets<RonAsset<LoaderFixture>>>();
    let asset = assets.get(id);
    assert!(
        asset.is_some(),
        "asset present per the wait above must still be present on re-read",
    );

    if let Some(asset) = asset {
        // `RonAsset` derefs to the payload — read the fields straight through.
        assert_eq!(
            asset.label, "grimdark",
            "deserialized `label` should equal the on-disk value",
        );
        assert_eq!(
            asset.count, 7,
            "deserialized `count` should equal the on-disk value",
        );
    }
}

/// A malformed loose `.ron` fails the load with a typed `LoadState::Failed`,
/// never a panic.
///
/// Pin-discriminating: if the loader `unwrap`ped / panicked on a parse error
/// (instead of returning the typed `RonLoadError`), this test would abort rather
/// than observe `Failed`. If a bad file silently resolved to a default value,
/// the `is_failed()` assert would fail. The fixture's `count: "not-a-number"`
/// cannot deserialize into the `u32` field, forcing the failure path.
#[test]
fn malformed_ron_fails_with_typed_load_state() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.init_ron_asset::<LoaderFixture>();

    let handle: Handle<RonAsset<LoaderFixture>> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load(MALFORMED_FIXTURE_PATH)
    };
    let id = handle.id();

    // Drive until the asset server reports a terminal failure for this id.
    let failed = gdtf_test_utils::advance_until(
        &mut app,
        |app| {
            app.world()
                .resource::<AssetServer>()
                .get_load_state(id)
                .is_some_and(|state| state.is_failed())
        },
        MAX_LOAD_UPDATES,
    );

    assert!(
        failed,
        "loading the malformed {MALFORMED_FIXTURE_PATH} should reach \
         LoadState::Failed within {MAX_LOAD_UPDATES} updates — proves the loader \
         surfaces a typed error rather than panicking or silently succeeding",
    );

    // And the asset must NOT have been inserted into the collection.
    assert!(
        app.world()
            .resource::<Assets<RonAsset<LoaderFixture>>>()
            .get(id)
            .is_none(),
        "a failed load must not insert an asset into the collection",
    );
}
