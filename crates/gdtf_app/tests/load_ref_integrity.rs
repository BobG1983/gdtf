//! GTW-582: the unified end-of-`Load` **reference-integrity pass** — one
//! dangling key per content-graph edge class in a fixture root must each land
//! as a specific finding in the [`ContentIntegrityReport`] (POSITIVE content
//! assertions), `Load` must still exit (validation is loud, never fatal), and
//! the SHIPPED `assets/` graph must validate with ZERO findings (graph
//! integrity, never magnitudes).
//!
//! Both tests drive the REAL `Load` chain over a real `AssetServer`
//! ([`GdtfLoadTestAppBuilder`]) — no shadow walker, no stubbed registries: the
//! findings come from the same per-edge check systems the production app runs.

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentValidationDone, ReferenceKeyScheme,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Snapshot the report's findings out of the world (empty when the resource is
/// absent — the following asserts then fail with an informative empty list).
fn findings_snapshot(app: &bevy::app::App) -> Vec<ContentFinding> {
    app.world()
        .get_resource::<ContentIntegrityReport>()
        .map(|report| report.findings().to_vec())
        .unwrap_or_default()
}

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a signal-poll cap, never a timing budget
/// (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The fixture root whose authored content dangles ONE reference per edge
/// class (`tests/fixtures/ref_integrity_root`), materializing only the
/// overridden content subdirs (the GTW-580 fixture convention; the
/// un-materialized folders fail closed to empty registries).
fn ref_integrity_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ref_integrity_root")
}

/// Whether the report carries a [`ContentFinding::DanglingRef`] whose target /
/// family / scheme match, with `referrer_hint` somewhere in its referrer — the
/// POSITIVE per-edge assertion shape.
fn has_dangling(
    findings: &[ContentFinding],
    referrer_hint: &str,
    target: &str,
    family: &str,
    scheme: ReferenceKeyScheme,
) -> bool {
    findings.iter().any(|finding| {
        matches!(
            finding,
            ContentFinding::DanglingRef {
                referrer: r,
                target: t,
                family: f,
                scheme: s,
            } if r.contains(referrer_hint) && ***t == *target && ***f == *family && *s == scheme
        )
    })
}

/// AC-1: over the dangling fixture root, the end-of-`Load` pass reports EVERY
/// edge class's specific finding — each assertion names the referencing
/// context, the dangling target, the target family, and (on the gang path)
/// which key SCHEME failed — and `Load` still exits to Intro-or-beyond (the
/// GTW-589 transient-Intro lesson; validation never strands the machine).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one POSITIVE assertion per C1 edge class over ONE Load-chain app spin-up; \
              splitting per edge would re-run the whole real async Load once per class"
)]
fn dangling_reference_per_edge_class_is_each_reported_and_load_still_exits() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(ref_integrity_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the pass itself: the publish stamps ContentValidationDone.
    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some(),
        "the unified validation pass must publish (ContentValidationDone) within the \
         safety net; last AppState was {:?}",
        app_state(&app),
    );

    let report = findings_snapshot(&app);

    // (1) situation → gang, by FILE STEM (the gang scheme).
    assert!(
        has_dangling(
            &report,
            "skirmish.ron",
            "ghost_gang",
            "GangRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the missing gang ref must be reported under the FILE-STEM scheme; findings: {report:?}",
    );
    // (2) situation → member, by roster DISPLAY-NAME (the member scheme).
    assert!(
        has_dangling(
            &report,
            "skirmish.ron",
            "Missing Member",
            "GangRegistry roster `fixture_gang`",
            ReferenceKeyScheme::DisplayName,
        ),
        "the missing member ref must be reported under the DISPLAY-NAME scheme; findings: {report:?}",
    );
    // (3) gang member → weapon / armor / melee keys.
    for (target, family) in [
        ("ghost_gun", "WeaponRegistry"),
        ("ghost_vest", "ArmorRegistry"),
        ("ghost_blade", "MeleeWeaponRegistry"),
    ] {
        assert!(
            has_dangling(
                &report,
                "fixture_gang",
                target,
                family,
                ReferenceKeyScheme::FileStem,
            ),
            "the member's dangling `{target}` key must be reported against {family}; \
             findings: {report:?}",
        );
    }
    // (4) weapon → attachment key (the formerly-silent drop, C3(b)).
    assert!(
        has_dangling(
            &report,
            "fixture_gun",
            "ghost_scope",
            "AttachmentRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the weapon's dangling attachment key must be reported; findings: {report:?}",
    );
    // (5) injury weighting → injury key (the warn-skip, now also reported, C3(c)).
    assert!(
        has_dangling(
            &report,
            "Torso",
            "ghost_injury",
            "InjuryRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the weighting row's dangling injury key must be reported; findings: {report:?}",
    );
    // (6) situation → theme UUID.
    assert!(
        has_dangling(
            &report,
            "skirmish.ron",
            "00000000-0000-0000-0000-058200000001",
            "UuidThemeRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the situation's dangling theme UUID must be reported; findings: {report:?}",
    );
    // (7) situation → terrain UUID (walls).
    assert!(
        has_dangling(
            &report,
            "walls",
            "00000000-0000-0000-0000-058200000002",
            "TerrainDefRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the situation's dangling wall terrain UUID must be reported; findings: {report:?}",
    );
    // (8) situation → field key.
    assert!(
        has_dangling(
            &report,
            "fields",
            "ghost_field",
            "FieldDefRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the situation's dangling field key must be reported; findings: {report:?}",
    );
    // (9) theme → terrain-def UUIDs (default_floor + palette).
    for target in [
        "00000000-0000-0000-0000-058200000005",
        "00000000-0000-0000-0000-058200000006",
    ] {
        assert!(
            has_dangling(
                &report,
                "Fixture Theme",
                target,
                "TerrainDefRegistry",
                ReferenceKeyScheme::Uuid,
            ),
            "the theme's dangling terrain UUID `{target}` must be reported; findings: {report:?}",
        );
    }
    // (10) prefab → theme UUID agreement + prefab → terrain-def UUID.
    assert!(
        has_dangling(
            &report,
            "broken_refs",
            "00000000-0000-0000-0000-058200000003",
            "UuidThemeRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the prefab's dangling theme UUID must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            &report,
            "broken_refs",
            "00000000-0000-0000-0000-058200000004",
            "TerrainDefRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the prefab's dangling placed terrain UUID must be reported; findings: {report:?}",
    );
    // (11) emplacement terrain def → mounted weapon key (the C1-walk edge).
    assert!(
        has_dangling(
            &report,
            "emplacement mounted_weapon",
            "ghost_cannon",
            "WeaponRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the emplacement's dangling mounted-weapon key must be reported; findings: {report:?}",
    );
    // (12) terrain def → sprite def by graphic_name (the GTW-663 foreign-key
    // edge; the fixture root materializes no content/sprites, so the authored
    // `ghost_graphic` key resolves nothing in the fail-closed EMPTY registry).
    assert!(
        has_dangling(
            &report,
            "Ghost Tile",
            "ghost_graphic",
            "SpriteDefRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the terrain def's dangling graphic_name must be reported; findings: {report:?}",
    );
    // The CLEAN placement ("Real Member" of fixture_gang) must NOT be reported —
    // the pass is discriminating, not noisy.
    assert!(
        !has_dangling(
            &report,
            "skirmish.ron",
            "Real Member",
            "GangRegistry roster `fixture_gang`",
            ReferenceKeyScheme::DisplayName,
        ),
        "a resolvable member ref must not be reported; findings: {report:?}",
    );

    // Validation NEVER strands Load: the machine still exits to Intro-or-beyond
    // (Intro is TRANSIENT under this harness — GTW-589).
    let load_released = advance_until(
        &mut app,
        |app| matches!(app_state(app), AppState::Intro | AppState::Running),
        LOAD_SAFETY_NET,
    );
    assert!(
        load_released,
        "Load must still exit with findings on the report (loud, never fatal); \
         last AppState was {:?}",
        app_state(&app),
    );
}

/// AC-4: the SHIPPED `assets/` content graph validates with ZERO findings —
/// pure graph integrity (every authored reference resolves), never a magnitude
/// pin, so authoring MORE content can never redden it (only a genuinely
/// dangling reference can).
#[test]
fn shipped_content_graph_validates_with_zero_findings() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<ContentValidationDone>(&mut app, LOAD_SAFETY_NET);
    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some(),
        "the unified validation pass must publish over the shipped assets/ root; \
         last AppState was {:?}",
        app_state(&app),
    );
    let report = findings_snapshot(&app);
    assert!(
        report.is_empty(),
        "the shipped content graph must carry ZERO dangling references; findings: {report:?}",
    );
}
