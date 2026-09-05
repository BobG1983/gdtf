//! wrapper opts in with `#[path = "load_suite/behaviors.rs"] mod behaviors;`
use std::path::PathBuf;

use bevy::asset::{AssetServer, Assets};
use cobalt_ron_assets::RonAsset;
use cobalt_test_utils::{LoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_assets::{ContentFamily, ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_game::test_support::{AppState, app_state, load_released};

const HOLDBACK_UPDATES: u32 = 4;

pub(crate) trait FamilyBehaviorContract: ContentFamily {
    const SALVAGE_FIXTURE_ROOT: &'static str;

    const SALVAGE_GOOD_MEMBERS: &'static [&'static str];

    const SALVAGE_BROKEN_FILE: &'static str;

    const MISSING_FOLDER_FIXTURE_ROOT: &'static str;

    const PROBE_MEMBER: &'static str;

    fn is_empty(registry: &Self::Registry) -> bool;

    fn member_resolves(registry: &Self::Registry, label: &str) -> bool;

    fn mutate_spec(spec: &mut Self::Spec);

    fn mutation_visible(registry: &Self::Registry, label: &str) -> bool;
}

pub(crate) fn salvage_parity<F: FamilyBehaviorContract>() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        fixture_root(F::SALVAGE_FIXTURE_ROOT),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<F::Registry>(&mut app);
    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "the broken `{}` folder must still resolve a {} (per-file salvage); last AppState \
         was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        for member in F::SALVAGE_GOOD_MEMBERS {
            assert!(
                F::member_resolves(registry, member),
                "the well-formed sibling `{member}` must be in the salvaged {} (fail-closed \
                 per FILE, not per family)",
                registry_name::<F>(),
            );
        }
        assert!(
            !F::is_empty(registry),
            "one malformed file may not EMPTY the {}",
            registry_name::<F>(),
        );
        let broken_stem = F::SALVAGE_BROKEN_FILE
            .strip_suffix(&format!(".{}", F::EXTENSION))
            .unwrap_or(F::SALVAGE_BROKEN_FILE);
        assert!(
            !F::member_resolves(registry, broken_stem),
            "the malformed member `{broken_stem}` must NOT resolve into the {}",
            registry_name::<F>(),
        );
    }

    advance_until_resource_exists::<ContentValidationDone>(&mut app);
    let reported = app
        .world()
        .get_resource::<ContentIntegrityReport>()
        .is_some_and(|report| {
            report.findings().iter().any(|finding| {
                matches!(
                    finding,
                    ContentFinding::MalformedFile { path, family, .. }
                        if path.contains(F::SALVAGE_BROKEN_FILE)
                            && **family == registry_name::<F>()
                )
            })
        });
    assert!(
        reported,
        "the malformed `{}` must be reported as a MalformedFile finding against the {}; \
         findings: {:?}",
        F::SALVAGE_BROKEN_FILE,
        registry_name::<F>(),
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .map(ContentIntegrityReport::findings),
    );

    assert_load_releases::<F>(&mut app, "the salvaged folder");
}

pub(crate) fn missing_folder_fails_closed_empty<F: FamilyBehaviorContract>() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        fixture_root(F::MISSING_FOLDER_FIXTURE_ROOT),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<F::Registry>(&mut app);
    let registry = app.world().get_resource::<F::Registry>();
    assert!(
        registry.is_some(),
        "a missing `{}` folder must still publish a {} (fail-closed, never a strand); last \
         AppState was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );
    if let Some(registry) = registry {
        assert!(
            F::is_empty(registry),
            "an unenumerable `{}` folder must fail closed to the EMPTY {}",
            F::FOLDER,
            registry_name::<F>(),
        );
    }

    assert_load_releases::<F>(&mut app, "the missing folder's empty registry");
}

pub(crate) fn never_publishes_partial<F: FamilyBehaviorContract>() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();
    advance_until_resource_exists::<F::Registry>(&mut app);
    assert!(
        app.world().get_resource::<F::Registry>().is_some(),
        "the real `{}` folder must resolve a {} before the partial-hold walk; last AppState \
         was {:?}",
        F::FOLDER,
        registry_name::<F>(),
        app_state(&app),
    );

    app.world_mut().remove_resource::<F::Registry>();
    let member = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<F::Spec>>(member_path::<F>());
    let removed = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<F::Spec>>>()
        .remove(member.id());
    assert!(
        removed.is_some(),
        "the `{}` member must have been resident to remove",
        member_path::<F>(),
    );

    for _ in 0..HOLDBACK_UPDATES {
        app.update();
    }
    assert!(
        app.world().get_resource::<F::Registry>().is_none(),
        "with `{}` missing from its collection, NOTHING may be published (no partial {})",
        member_path::<F>(),
        registry_name::<F>(),
    );

    let Some(spec) = removed else { return };
    let reinserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<F::Spec>>>()
        .insert(member.id(), spec);
    assert!(reinserted.is_ok(), "re-inserting the member must succeed");
    advance_until_resource_exists::<F::Registry>(&mut app);
    let resolves = app
        .world()
        .get_resource::<F::Registry>()
        .is_some_and(|registry| F::member_resolves(registry, F::PROBE_MEMBER));
    assert!(
        resolves,
        "once `{}` returns, the retrying resolve must publish the FULL {} (member resolving)",
        member_path::<F>(),
        registry_name::<F>(),
    );
}

pub(crate) fn redrive_rebuilds_live<F: FamilyBehaviorContract>() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();
    advance_until_resource_exists::<F::Registry>(&mut app);

    let member = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<F::Spec>>(member_path::<F>());
    let mutated = {
        let mut specs = app.world_mut().resource_mut::<Assets<RonAsset<F::Spec>>>();
        match specs.get_mut(&member) {
            Some(mut asset) => {
                F::mutate_spec(&mut **asset);
                true
            }
            None => false,
        }
    };
    assert!(
        mutated,
        "the `{}` member must be resident to hot-edit",
        member_path::<F>(),
    );

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<F::Registry>()
            .is_some_and(|registry| F::mutation_visible(registry, F::PROBE_MEMBER))
    });
}

fn assert_load_releases<F: FamilyBehaviorContract>(app: &mut bevy::app::App, _scenario: &str) {
    advance_until(app, load_released);
}

fn registry_name<F: FamilyBehaviorContract>() -> &'static str {
    let full = std::any::type_name::<F::Registry>();
    full.rsplit("::").next().unwrap_or(full)
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn member_path<F: FamilyBehaviorContract>() -> String {
    format!("{}/{}.{}", F::FOLDER, F::PROBE_MEMBER, F::EXTENSION)
}
