use std::collections::HashMap;

use bevy::{asset::Assets, prelude::*, reflect::TypePath};
use gdtf_assets::{
    ContentFamily, ContentFamilyAppExt, ContentFileStem, ContentFolderHandle, RonAsset,
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

#[derive(Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct Swatch {
        tone: u32,
}

#[derive(Resource, Default, Debug, PartialEq, Eq)]
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

#[derive(Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct Badge {
        key:   String,
        glyph: String,
}

#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct BadgeRegistry(HashMap<String, String>);

struct BadgeFamily;

impl ContentFamily for BadgeFamily {
    type Spec = Badge;
    type Registry = BadgeRegistry;

    const EXTENSION: &'static str = "badge.ron";
    const FOLDER: &'static str = "test/content_family";

    fn insert_member(registry: &mut BadgeRegistry, _stem: Option<ContentFileStem>, spec: &Badge) {
        registry.0.insert(spec.key.clone(), spec.glyph.clone());
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

#[derive(Resource, Default, Debug, PartialEq, Eq)]
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

fn real_asset_app() -> App {
    GdtfUiTestAppBuilder::new().with_ui_camera().build()
}

#[test]
fn mixed_folder_resolves_both_keying_shapes_and_skips_wrong_typed_members() {
    let mut app = real_asset_app();
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();

    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );
    gdtf_test_utils::advance_until_resource_exists::<BadgeRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    let mut expected_swatches = HashMap::new();
    expected_swatches.insert("alpha".to_owned(), 3);
    expected_swatches.insert("beta".to_owned(), 7);
    assert_eq!(
        app.world().get_resource::<SwatchRegistry>(),
        Some(&SwatchRegistry(expected_swatches)),
        "the stem-keyed family must key by infix-stripped stems and SKIP the badge member",
    );

    let mut expected_badges = HashMap::new();
    expected_badges.insert("brand_theta".to_owned(), "θ".to_owned());
    assert_eq!(
        app.world().get_resource::<BadgeRegistry>(),
        Some(&BadgeRegistry(expected_badges)),
        "the payload-keyed family must key by the id INSIDE the payload and SKIP the swatches",
    );

    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<SwatchFamily>>()
            .is_some(),
        "the kick-off must have stored the persistent swatch folder handle",
    );
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<BadgeFamily>>()
            .is_some(),
        "the kick-off must have stored the persistent badge folder handle",
    );
}

#[test]
fn failed_folder_inserts_the_empty_registry() {
    let mut app = real_asset_app();
    app.register_content_family::<RelicFamily>();

    gdtf_test_utils::advance_until_resource_exists::<RelicRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    assert_eq!(
        app.world().get_resource::<RelicRegistry>(),
        Some(&RelicRegistry(HashMap::new())),
        "a Failed folder must fail closed into the EMPTY registry, never hang the gate",
    );
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<RelicFamily>>()
            .is_some(),
        "the persistent folder handle must exist even when the folder load failed",
    );
}

#[test]
fn missing_member_publishes_nothing_until_it_returns() {
    let mut app = real_asset_app();
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    app.world_mut().remove_resource::<SwatchRegistry>();
    let beta = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<Swatch>>("test/content_family/beta.swatch.ron");
    let removed = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<Swatch>>>()
        .remove(beta.id());
    assert!(
        removed.is_some(),
        "the beta member must have been resident to remove",
    );

    for _ in 0..4 {
        app.update();
    }
    assert!(
        app.world().get_resource::<SwatchRegistry>().is_none(),
        "with a member missing from its collection, NOTHING may be published (no partial registry)",
    );

    let reinserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<Swatch>>>()
        .insert(beta.id(), RonAsset::new(Swatch { tone: 7 }));
    assert!(reinserted.is_ok(), "re-inserting the member must succeed");
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );
    let betas = app
        .world()
        .get_resource::<SwatchRegistry>()
        .and_then(|registry| registry.0.get("beta").copied());
    assert_eq!(
        betas,
        Some(7),
        "once the member returns, the retrying resolve must publish the FULL registry",
    );
}

#[test]
fn modified_member_rebuilds_the_registry_live() {
    let mut app = real_asset_app();
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
        **asset = Swatch { tone: 9 };
    }

    let rebuilt = gdtf_test_utils::advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<SwatchRegistry>()
                .and_then(|registry| registry.0.get("alpha").copied())
                == Some(9)
        },
        GENEROUS_LOAD_UPDATES,
    );
    assert!(
        rebuilt,
        "a Modified member must rebuild the registry in place with the edited tone",
    );
}

#[test]
fn headless_minimal_app_seeds_the_default_registry_and_skips_the_chain() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.register_content_family::<SwatchFamily>();

    for _ in 0..3 {
        app.update();
    }

    let registry = app.world().get_resource::<SwatchRegistry>();
    assert!(
        registry.is_some(),
        "a headless registration must seed the DEFAULT registry (the headless fallback)",
    );
    if let Some(registry) = registry {
        assert!(
            registry.0.is_empty(),
            "the headless fallback is the EMPTY default — nothing resolves it",
        );
    }
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<SwatchFamily>>()
            .is_none(),
        "a headless app must never kick off the folder load",
    );
}
