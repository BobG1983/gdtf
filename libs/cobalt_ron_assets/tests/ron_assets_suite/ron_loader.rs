//! RON loader: well-formed resolves; malformed fails without inserting.
use bevy::{
    asset::{AssetServer, Assets, Handle},
    reflect::TypePath,
};
use cobalt_ron_assets::{RonAsset, RonAssetAppExt};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

#[derive(Deserialize, TypePath, Debug, PartialEq, Eq)]
struct LoaderFixture {
    label: String,
    count: u32,
}

const GOOD_FIXTURE_PATH: &str = "test/ron_loader_fixture.ron";

const MALFORMED_FIXTURE_PATH: &str = "test/ron_loader_malformed.ron";

#[test]
fn well_formed_ron_resolves_to_typed_asset() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.init_ron_asset::<LoaderFixture>();

    let handle: Handle<RonAsset<LoaderFixture>> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load(GOOD_FIXTURE_PATH)
    };

    let id = handle.id();
    gdtf_test_utils::advance_until_load_state(&mut app, id, |state| state.is_loaded());

    let assets = app.world().resource::<Assets<RonAsset<LoaderFixture>>>();
    let asset = assets.get(id);
    assert!(
        asset.is_some(),
        "asset present per the wait above must still be present on re-read",
    );

    if let Some(asset) = asset {
        assert_eq!(
            asset.label, "fixture-label",
            "deserialized `label` should equal the on-disk value",
        );
        assert_eq!(
            asset.count, 7,
            "deserialized `count` should equal the on-disk value",
        );
    }
}

#[test]
fn malformed_ron_fails_with_typed_load_state() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.init_ron_asset::<LoaderFixture>();

    let handle: Handle<RonAsset<LoaderFixture>> = {
        let asset_server = app.world().resource::<AssetServer>();
        asset_server.load(MALFORMED_FIXTURE_PATH)
    };
    let id = handle.id();

    gdtf_test_utils::advance_until_load_state(&mut app, id, |state| state.is_failed());

    assert!(
        app.world()
            .resource::<Assets<RonAsset<LoaderFixture>>>()
            .get(id)
            .is_none(),
        "a failed load must not insert an asset into the collection",
    );
}
