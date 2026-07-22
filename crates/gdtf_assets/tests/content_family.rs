//! Integration tests for the GTW-570 content-family machinery — the ext-registered
//! kick-off / resolve / redrive chain on a REAL `AssetServer` + the on-disk
//! fixture folder (`assets/test/content_family/`).
//!
//! The `Swatch` / `Badge` families below ARE the contract's add-one-family
//! demonstration: one payload type, one [`ContentFamily`] marker impl, and one
//! [`register_content_family`](ContentFamilyAppExt::register_content_family)
//! call is ALL a new folder family needs — no per-site resolve module, no poll
//! branch, no handle newtypes, no redrive clone. The fixture folder is MIXED
//! (two swatch members + one badge member) so the unconditional `TypeId`
//! member filter and BOTH keying shapes (stem vs payload) are exercised on the
//! real code path. The reload / fallback LOG lines are asserted separately in
//! `content_family_logs.rs` (the thread-routed capture needs its own binary).

use std::collections::HashMap;

use bevy::{asset::Assets, prelude::*, reflect::TypePath};
use gdtf_assets::{
    ContentFamily, ContentFamilyAppExt, ContentFileStem, ContentFolderHandle, RonAsset,
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use serde::Deserialize;

/// The STEM-KEYED test payload (`*.swatch.ron`).
#[derive(Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct Swatch {
    /// A distinguishable magnitude the tests assert verbatim.
    tone: u32,
}

/// The stem-keyed registry the swatch folder resolves into.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct SwatchRegistry(HashMap<String, u32>);

/// The stem-keyed test family: `test/content_family/*.swatch.ron` →
/// [`SwatchRegistry`], keyed by the infix-stripped file stem.
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

/// The PAYLOAD-KEYED test payload (`*.badge.ron`) — its registry key lives
/// INSIDE the file (the terrain/theme-def shape).
#[derive(Deserialize, TypePath, Debug, Clone, PartialEq, Eq)]
struct Badge {
    /// The payload-owned registry key (the filename stem is irrelevant).
    key:   String,
    /// A payload field the tests read back.
    glyph: String,
}

/// The payload-keyed registry the badge members resolve into.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct BadgeRegistry(HashMap<String, String>);

/// The payload-keyed test family sharing the SAME mixed folder as
/// [`SwatchFamily`].
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

/// A payload for the FAILED-folder family (its folder does not exist, so no
/// member ever parses — the type only anchors the loader registration).
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
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct RelicRegistry(HashMap<String, u32>);

/// A family whose folder is deliberately MISSING — drives the fail-closed path.
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

/// A real-`AssetServer` app (`DefaultPlugins`, backends: None, asset source
/// rooted at the workspace `assets/`) — the same harness the hot-RON tests
/// drive.
fn real_asset_app() -> App {
    GdtfUiTestAppBuilder::new().with_ui_camera().build()
}

/// C1 / C2(c-d) / C7: BOTH keying shapes resolve from the ONE mixed folder
/// through the real ext-registered chain, each skipping the other family's
/// members via the UNCONDITIONAL `TypeId` filter.
///
/// Pin-discriminating: without the `TypeId` filter the walk debug-panics typing
/// `theta.badge.ron` as a `RonAsset<Swatch>` (or never resolves); a mis-keyed
/// stem walk drops the `alpha`/`beta` keys (the un-stripped stem would be
/// `alpha.swatch`); a stem-keyed badge walk would key `theta`, not the
/// payload's `brand_theta`.
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

    // C2(d): the persistent generic folder handle sits beside each registry
    // (never removed), feeding the redrive + keeping members alive for the
    // file-watcher.
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

/// C2(b) / C7: a folder that reaches a genuine `Failed` (the directory does
/// not exist) fails CLOSED — the chain publishes the EMPTY registry so a
/// presence-gated Load flow is never stranded (ADR-0003). The matching `warn!`
/// is asserted in `content_family_logs.rs`.
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
    // The persistent handle exists even on the failure path (the kick-off
    // inserted it), so a later file-watcher recovery can re-enumerate.
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<RelicFamily>>()
            .is_some(),
        "the persistent folder handle must exist even when the folder load failed",
    );
}

/// C2(c) / C7: while ANY matching-type member is absent from its `Assets`
/// collection the resolve publishes NOTHING and retries — a partial registry
/// is never observable; once the member returns, the retry publishes the full
/// registry.
///
/// Driven through the REAL ext-registered resolve: after a genuine resolve,
/// the registry is removed (re-arming the absence-gated system) and one member
/// asset is pulled out of the collection — the folder's load state stays
/// `Loaded`, so only the never-publish-partial walk can hold the registry back.
#[test]
fn missing_member_publishes_nothing_until_it_returns() {
    let mut app = real_asset_app();
    // BOTH families register (the folder is mixed — every member extension
    // needs a loader for the recursive folder load to reach `Loaded`).
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    // Re-arm the resolve (absence-gated) and knock ONE member out of the
    // collection (the folder handle + load state are untouched).
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

    // The member returns — the still-alive resolve retries and publishes the
    // FULL registry.
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

/// C2(e): a member `Modified` event rebuilds the resident registry IN PLACE
/// from the latest in-memory specs — the live hot-reload, through the real
/// ext-registered redrive. (The Part C `info!` line is asserted in
/// `content_family_logs.rs`.)
///
/// Pin-discriminating: dropping the rebuild leaves the OLD tone.
#[test]
fn modified_member_rebuilds_the_registry_live() {
    let mut app = real_asset_app();
    // BOTH families register (the folder is mixed — every member extension
    // needs a loader for the recursive folder load to reach `Loaded`).
    app.register_content_family::<SwatchFamily>();
    app.register_content_family::<BadgeFamily>();
    gdtf_test_utils::advance_until_resource_exists::<SwatchRegistry>(
        &mut app,
        GENEROUS_LOAD_UPDATES,
    );

    // Hot-edit alpha's payload in place; `get_mut` queues the Modified event
    // the ungated redrive reacts to (the file-watcher stand-in).
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

/// C2(a) / C7 + the GTW-629 headless-fallback rider: a no-`AssetServer`
/// `MinimalPlugins` app updates without panic — registration self-gates, so
/// the ext call registers NO chain (no loader, no systems, no handle) but
/// seeds the family's DEFAULT registry as the headless fallback, so a
/// presence-gated host flow (the game's Load gate) stays satisfiable from the
/// ONE registration line, with zero per-family seed arms anywhere else.
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
        "a headless registration must seed the DEFAULT registry (the GTW-629 fallback rider)",
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
