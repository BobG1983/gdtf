//! Per-file salvage: malformed siblings reported; good members still load; Load still exits.
use std::path::PathBuf;

use cobalt_test_utils::{LoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    effects::fields::{FieldDefRegistry, FieldKey},
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_game::test_support::{AppState, app_state};

fn salvage_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("salvage_root")
}

#[test]
fn malformed_sibling_is_salvaged_around_reported_and_load_still_exits() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        salvage_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app);

    let weapons = app.world().get_resource::<WeaponRegistry>();
    assert!(
        weapons.is_some(),
        "the weapons folder must still resolve a WeaponRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(weapons) = weapons {
        for key in ["good_a", "good_b"] {
            assert!(
                weapons.spec(&WeaponName::new(key.to_owned())).is_some(),
                "the well-formed sibling `{key}` must be in the salvaged registry \
                 (fail-closed per FILE, not per family)",
            );
        }
        assert!(
            weapons
                .spec(&WeaponName::new("broken".to_owned()))
                .is_none(),
            "the malformed member must NOT resolve into the registry",
        );
        assert!(
            !weapons.is_empty(),
            "one malformed file may no longer EMPTY its family registry",
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
                        if path.contains("broken.weapon.ron") && ***family == *"WeaponRegistry"
                )
            })
        });
    assert!(
        reported,
        "the malformed `broken.weapon.ron` must be reported as a MalformedFile finding \
         against the WeaponRegistry; findings: {:?}",
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .map(ContentIntegrityReport::findings),
    );

    advance_until(&mut app, |app| {
        matches!(app_state(app), AppState::Intro | AppState::Running)
    });
}

#[test]
fn an_authored_zero_turn_dot_is_salvage_rejected_and_never_reaches_the_sim() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        salvage_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app);

    let weapons = app.world().get_resource::<WeaponRegistry>();
    assert!(
        weapons.is_some(),
        "the weapons folder must still resolve a WeaponRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(weapons) = weapons {
        assert!(
            weapons
                .spec(&WeaponName::new("zero_turn_dot".to_owned()))
                .is_none(),
            "a weapon authoring a zero-turn DOT must NOT resolve into the registry \
             (a zero-turn DOT is unrepresentable; no silent clamp-to-1)",
        );
        for key in ["good_a", "good_b"] {
            assert!(
                weapons.spec(&WeaponName::new(key.to_owned())).is_some(),
                "the well-formed sibling `{key}` must still be in the salvaged registry",
            );
        }
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
                        if path.contains("zero_turn_dot.weapon.ron")
                            && ***family == *"WeaponRegistry"
                )
            })
        });
    assert!(
        reported,
        "the authored zero-turn DOT must be reported as a MalformedFile finding \
         against the WeaponRegistry; findings: {:?}",
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .map(ContentIntegrityReport::findings),
    );
}

#[test]
fn an_authored_zero_turn_field_is_salvage_rejected_and_never_reaches_the_sim() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        salvage_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<FieldDefRegistry>(&mut app);

    let fields = app.world().get_resource::<FieldDefRegistry>();
    assert!(
        fields.is_some(),
        "the fields folder must still resolve a FieldDefRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(fields) = fields {
        assert!(
            fields
                .def(&FieldKey::new("zero_turn_field".to_owned()))
                .is_none(),
            "a field authoring `duration: Turns(0)` must NOT resolve into the registry \
             (a zero-turn field is unrepresentable; no silent clamp-to-1)",
        );
        assert!(
            fields
                .def(&FieldKey::new("caustic_pond".to_owned()))
                .is_some(),
            "the well-formed sibling `caustic_pond` must still be in the salvaged registry",
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
                        if path.contains("zero_turn_field.field.ron")
                            && ***family == *"FieldDefRegistry"
                )
            })
        });
    assert!(
        reported,
        "the authored zero-turn field must be reported as a MalformedFile finding \
         against the FieldDefRegistry; findings: {:?}",
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .map(ContentIntegrityReport::findings),
    );
}
