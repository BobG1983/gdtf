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
//! Drives the REAL `Load` chain over a real `AssetServer`
//! ([`GdtfLoadTestAppBuilder`]) — the salvage under test is the one the
//! production seam runs.

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state};
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits — a
/// signal-poll cap, never a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The fixture root whose `content/weapons/ranged/` carries 2 well-formed
/// weapons + 1 deliberately-malformed `broken.weapon.ron`
/// (`tests/fixtures/salvage_root`), materializing only that overridden subdir
/// (the GTW-580 fixture convention).
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
