//! Content family: mixed folder, fail-closed empty, no partial publish, live rebuild,
//! and the source path recorded beside every registry key.
use std::{collections::HashMap, path::PathBuf};

use bevy::{asset::Assets, prelude::*, reflect::TypePath};
use cobalt_ron_assets::RonAsset;
use cobalt_test_utils::UiTestAppBuilder;
use gdtf_assets::{
    ContentFamily, ContentFamilyAppExt, ContentFileStem, ContentFolderHandle, ContentMemberKey,
    ContentSourcePaths,
};
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

    fn insert_member(
        registry: &mut SwatchRegistry,
        stem: Option<ContentFileStem>,
        spec: &Swatch,
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.0.insert(key.clone(), spec.tone);
        Some(ContentMemberKey::new(key))
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

    fn insert_member(
        registry: &mut BadgeRegistry,
        _stem: Option<ContentFileStem>,
        spec: &Badge,
    ) -> Option<ContentMemberKey> {
        registry.0.insert(spec.key.clone(), spec.glyph.clone());
        Some(ContentMemberKey::new(spec.key.clone()))
    }
}

#[derive(Deserialize, TypePath, Debug, Clone)]
struct Relic {
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

    fn insert_member(
        registry: &mut RelicRegistry,
        stem: Option<ContentFileStem>,
        spec: &Relic,
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.0.insert(key.clone(), spec.age);
        Some(ContentMemberKey::new(key))
    }
}

fn real_asset_app() -> App {
    UiTestAppBuilder::new().with_ui_camera().build()
}

#[test]
fn mixed_folder_resolves_both_keying_shapes_and_skips_wrong_typed_members() {
    let mut app = real_asset_app();
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();

    cobalt_test_utils::advance_until_resource_exists::<SwatchRegistry>(&mut app);
    cobalt_test_utils::advance_until_resource_exists::<BadgeRegistry>(&mut app);

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

    cobalt_test_utils::advance_until_resource_exists::<RelicRegistry>(&mut app);

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
    cobalt_test_utils::advance_until_resource_exists::<SwatchRegistry>(&mut app);

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
    cobalt_test_utils::advance_until_resource_exists::<SwatchRegistry>(&mut app);
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
    cobalt_test_utils::advance_until_resource_exists::<SwatchRegistry>(&mut app);

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

    cobalt_test_utils::advance_until(&mut app, |app| {
        app.world()
            .get_resource::<SwatchRegistry>()
            .and_then(|registry| registry.0.get("alpha").copied())
            == Some(9)
    });
}

// Every key the swatch registry holds, so the path table is checked against the real set.
fn swatch_keys(app: &App) -> Vec<String> {
    app.world()
        .get_resource::<SwatchRegistry>()
        .map(|registry| registry.0.keys().cloned().collect())
        .unwrap_or_default()
}

// Whether the path table answers for one key, read without holding a world borrow.
fn swatch_source(app: &App, key: &str) -> Option<PathBuf> {
    app.world()
        .get_resource::<ContentSourcePaths<SwatchFamily>>()
        .and_then(|sources| sources.path(&ContentMemberKey::new(key.to_owned())))
        .map(|path| (**path).clone())
}

#[test]
fn every_registry_key_answers_the_file_it_was_read_from() {
    let mut app = real_asset_app();
    app.register_content_family::<SwatchFamily>();
    cobalt_test_utils::advance_until_resource_exists::<SwatchRegistry>(&mut app);

    let keys = swatch_keys(&app);
    assert!(
        !keys.is_empty(),
        "the swatch folder holds members, or the walk below checks nothing",
    );

    let mut seen: Vec<PathBuf> = Vec::new();
    for key in &keys {
        let Some(path) = swatch_source(&app, key) else {
            unreachable!("`{key}` is in the registry, so the path table must answer for it")
        };
        let starts_with_key = path
            .file_name()
            .map(|name| name.to_string_lossy().starts_with(key.as_str()));
        assert_eq!(
            starts_with_key,
            Some(true),
            "`{key}` must answer the file its own stem names, got {path:?}",
        );
        assert!(
            !seen.contains(&path),
            "two keys answered {path:?}; a save through one key would overwrite the other",
        );
        seen.push(path);
    }
    assert_eq!(
        swatch_source(&app, "no_such_swatch"),
        None,
        "a key no member carries answers nothing, so no caller writes to an invented path",
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
    cobalt_test_utils::advance_until(&mut app, |app| {
        app.world()
            .get_resource::<SwatchRegistry>()
            .and_then(|registry| registry.0.get("alpha").copied())
            == Some(9)
    });

    assert!(
        swatch_source(&app, "alpha").is_some(),
        "the hot rebuild replaces the path table beside the registry; a rebuild that replaces \
         only the registry leaves an author saving to a path nothing holds",
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
