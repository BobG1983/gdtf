//! GTW-582 C4 regression pin: **fail-closed per FILE, not per family** — a
//! content folder carrying ONE malformed `.ron` beside N well-formed siblings
//! must still resolve a registry holding the N siblings; the malformed file
//! alone fails, loudly, as a [`ContentFinding::MalformedFile`] on the report;
//! and `Load` still exits with every registry present.
//!
//! Pre-GTW-582 this exact fixture emptied the whole `WeaponRegistry` (Bevy's
//! `load_folder` fails the folder's `RecursiveDependencyLoadState` on the first
//! malformed member and the resolve fail-closed to the EMPTY registry) — this
//! test is RED on that behavior and GREEN on the per-file salvage.
//!
//! Also pins the GTW-643 authored-zero DOT rejection: a weapon `.ron` whose
//! `dot:` profile authors `turns: 0` must FAIL DESERIALIZATION LOUDLY (a
//! zero-turn DOT is unrepresentable — `DotTurns` wraps `NonZeroU8`) and flow
//! through the SAME per-file salvage into a
//! [`ContentFinding::MalformedFile`] — the sim never sees it, and there is no
//! silent clamp-to-1.
//!
//! And the GTW-659 authored-zero FIELD rejection, the same shape one family
//! over: a `.field.ron` authoring `duration: Turns(0)` must fail its per-file
//! load (a zero-turn field is unrepresentable — `FieldTurns` wraps
//! `NonZeroU8`), land as a [`ContentFinding::MalformedFile`], and never
//! resolve into the [`FieldDefRegistry`] — pre-GTW-659 it parsed and reached
//! the sim, where it behaved as `Turns(1)` (the authored value lied by one).
//!
//! Drives the REAL `Load` chain over a real `AssetServer`
//! ([`GdtfLoadTestAppBuilder`]) — the salvage under test is the one the
//! production loader runs.

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state};
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    effects::fields::{FieldDefRegistry, FieldKey},
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits — a
/// signal-poll cap, never a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The fixture root whose `content/weapons/ranged/` carries 2 well-formed
/// weapons + 1 deliberately-malformed `broken.weapon.ron` + 1 authored
/// zero-turn-DOT `zero_turn_dot.weapon.ron` (GTW-643), and whose
/// `content/fields/` carries 1 well-formed `caustic_pond.field.ron` + 1
/// authored zero-turn `zero_turn_field.field.ron` (GTW-659)
/// (`tests/fixtures/salvage_root`), materializing only those overridden
/// subdirs (the GTW-580 fixture convention).
fn salvage_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("salvage_root")
}

/// C4: the malformed member no longer empties its family — the registry holds
/// BOTH well-formed siblings, the malformed file is on the report, the
/// registry is NOT empty, and `Load` still exits.
#[test]
fn malformed_sibling_is_salvaged_around_reported_and_load_still_exits() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(salvage_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the weapons registry arrives via the per-file salvage path
    // (the folder's own load_folder walk FAILED on the malformed member).
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    let weapons = app.world().get_resource::<WeaponRegistry>();
    assert!(
        weapons.is_some(),
        "the weapons folder must still resolve a WeaponRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(weapons) = weapons {
        // The two WELL-FORMED siblings loaded (positive content assertions).
        for key in ["good_a", "good_b"] {
            assert!(
                weapons.spec(&WeaponName::new(key.to_owned())).is_some(),
                "the well-formed sibling `{key}` must be in the salvaged registry \
                 (fail-closed per FILE, not per family)",
            );
        }
        // The malformed member alone is missing — and the registry is NOT empty.
        assert!(
            weapons
                .spec(&WeaponName::new("broken".to_owned()))
                .is_none(),
            "the malformed member must NOT resolve into the registry",
        );
        assert!(
            !weapons.is_empty(),
            "one malformed file may no longer EMPTY its family registry (the pre-GTW-582 \
             behavior this test pins against)",
        );
    }

    // The malformed file is LOUD: a MalformedFile finding on the report naming it.
    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
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

    // Load still always exits with every registry present (Intro is transient —
    // GTW-589).
    let load_released = advance_until(
        &mut app,
        |app| matches!(app_state(app), AppState::Intro | AppState::Running),
        LOAD_SAFETY_NET,
    );
    assert!(
        load_released,
        "Load must still exit past the salvaged folder; last AppState was {:?}",
        app_state(&app),
    );
}

/// GTW-643 (A2): an authored ZERO-turn DOT is rejected at deserialization — the
/// `zero_turn_dot.weapon.ron` fixture (well-formed RON, `dot: Some((… turns: 0))`)
/// must fail its per-file load LOUDLY, land as a [`ContentFinding::MalformedFile`]
/// naming it on the report, and never resolve into the `WeaponRegistry` (the sim
/// never sees it — no silent clamp-to-1).
#[test]
fn an_authored_zero_turn_dot_is_salvage_rejected_and_never_reaches_the_sim() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(salvage_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the weapons registry arrives via the per-file salvage path.
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    let weapons = app.world().get_resource::<WeaponRegistry>();
    assert!(
        weapons.is_some(),
        "the weapons folder must still resolve a WeaponRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(weapons) = weapons {
        // The sim NEVER sees the zero-turn DOT weapon — rejected, not clamped in.
        assert!(
            weapons
                .spec(&WeaponName::new("zero_turn_dot".to_owned()))
                .is_none(),
            "a weapon authoring a zero-turn DOT must NOT resolve into the registry \
             (a zero-turn DOT is unrepresentable; no silent clamp-to-1)",
        );
        // The rejection is per-FILE: its well-formed siblings still load.
        for key in ["good_a", "good_b"] {
            assert!(
                weapons.spec(&WeaponName::new(key.to_owned())).is_some(),
                "the well-formed sibling `{key}` must still be in the salvaged registry",
            );
        }
    }

    // The rejection is LOUD: a MalformedFile finding on the report naming the file.
    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
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

/// GTW-659 (C1/A1): an authored ZERO-turn field duration is rejected at
/// deserialization — the `zero_turn_field.field.ron` fixture (well-formed RON,
/// `duration: Turns(0)`) must fail its per-file load LOUDLY, land as a
/// [`ContentFinding::MalformedFile`] naming it on the report, and never resolve
/// into the [`FieldDefRegistry`] (the sim never sees it — no silent clamp-to-1),
/// while its well-formed `caustic_pond` sibling still loads (per-FILE salvage).
#[test]
fn an_authored_zero_turn_field_is_salvage_rejected_and_never_reaches_the_sim() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(salvage_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the fields catalog arrives via the per-file salvage path.
    advance_until_resource_exists::<FieldDefRegistry>(&mut app, LOAD_SAFETY_NET);

    let fields = app.world().get_resource::<FieldDefRegistry>();
    assert!(
        fields.is_some(),
        "the fields folder must still resolve a FieldDefRegistry (per-file salvage); \
         last AppState was {:?}",
        app_state(&app),
    );
    if let Some(fields) = fields {
        // The sim NEVER sees the zero-turn field — rejected, not clamped in.
        assert!(
            fields
                .def(&FieldKey::new("zero_turn_field".to_owned()))
                .is_none(),
            "a field authoring `duration: Turns(0)` must NOT resolve into the registry \
             (a zero-turn field is unrepresentable; no silent clamp-to-1)",
        );
        // The rejection is per-FILE: the well-formed sibling still loads.
        assert!(
            fields
                .def(&FieldKey::new("caustic_pond".to_owned()))
                .is_some(),
            "the well-formed sibling `caustic_pond` must still be in the salvaged registry",
        );
    }

    // The rejection is LOUD: a MalformedFile finding on the report naming the file.
    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
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
