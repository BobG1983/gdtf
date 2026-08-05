//! Ref integrity: each dangling edge class is reported; shipped graph is clean.
use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentValidationDone, ReferenceKeyScheme,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

fn findings_snapshot(app: &bevy::app::App) -> Vec<ContentFinding> {
    app.world()
        .get_resource::<ContentIntegrityReport>()
        .map(|report| report.findings().to_vec())
        .unwrap_or_default()
}

const LOAD_SAFETY_NET: u32 = 10_000;

fn ref_integrity_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ref_integrity_root")
}

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

fn assert_situation_edges(report: &[ContentFinding]) {
    assert!(
        has_dangling(
            report,
            "skirmish.ron",
            "ghost_gang",
            "GangRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the missing gang ref must be reported under the FILE-STEM scheme; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "skirmish.ron",
            "Missing Member",
            "GangRegistry roster `fixture_gang`",
            ReferenceKeyScheme::DisplayName,
        ),
        "the missing member ref must be reported under the DISPLAY-NAME scheme; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "skirmish.ron",
            "00000000-0000-0000-0000-058200000001",
            "UuidThemeRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the situation's dangling theme UUID must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "walls",
            "00000000-0000-0000-0000-058200000002",
            "TerrainDefRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the situation's dangling wall terrain UUID must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "fields",
            "ghost_field",
            "FieldDefRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the situation's dangling field key must be reported; findings: {report:?}",
    );
}

fn assert_roster_loadout_edges(report: &[ContentFinding]) {
    for (target, family) in [
        ("ghost_gun", "WeaponRegistry"),
        ("ghost_vest", "ArmorRegistry"),
        ("ghost_blade", "MeleeWeaponRegistry"),
    ] {
        assert!(
            has_dangling(
                report,
                "fixture_gang",
                target,
                family,
                ReferenceKeyScheme::FileStem,
            ),
            "the member's dangling `{target}` key must be reported against {family}; \
             findings: {report:?}",
        );
    }
    assert!(
        !has_dangling(
            report,
            "skirmish.ron",
            "Real Member",
            "GangRegistry roster `fixture_gang`",
            ReferenceKeyScheme::DisplayName,
        ),
        "a resolvable member ref must not be reported; findings: {report:?}",
    );
}

fn assert_item_edges(report: &[ContentFinding]) {
    assert!(
        has_dangling(
            report,
            "fixture_gun",
            "ghost_scope",
            "AttachmentRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the weapon's dangling attachment key must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "Torso",
            "ghost_injury",
            "InjuryRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the weighting row's dangling injury key must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "emplacement mounted_weapon",
            "ghost_cannon",
            "WeaponRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the emplacement's dangling mounted-weapon key must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "Ghost Tile",
            "ghost_graphic",
            "SpriteDefRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "the terrain def's dangling graphic_name must be reported; findings: {report:?}",
    );
}

fn assert_theme_and_prefab_edges(report: &[ContentFinding]) {
    for target in [
        "00000000-0000-0000-0000-058200000005",
        "00000000-0000-0000-0000-058200000006",
    ] {
        assert!(
            has_dangling(
                report,
                "Fixture Theme",
                target,
                "TerrainDefRegistry",
                ReferenceKeyScheme::Uuid,
            ),
            "the theme's dangling terrain UUID `{target}` must be reported; findings: {report:?}",
        );
    }
    assert!(
        has_dangling(
            report,
            "broken_refs",
            "00000000-0000-0000-0000-058200000003",
            "UuidThemeRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the prefab's dangling theme UUID must be reported; findings: {report:?}",
    );
    assert!(
        has_dangling(
            report,
            "broken_refs",
            "00000000-0000-0000-0000-058200000004",
            "TerrainDefRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "the prefab's dangling placed terrain UUID must be reported; findings: {report:?}",
    );
}

#[test]
fn dangling_reference_per_edge_class_is_each_reported_and_load_still_exits() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(ref_integrity_root())
        .starting_in(AppState::Load)
        .build();

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

    assert_situation_edges(&report);
    assert_roster_loadout_edges(&report);
    assert_item_edges(&report);
    assert_theme_and_prefab_edges(&report);

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
