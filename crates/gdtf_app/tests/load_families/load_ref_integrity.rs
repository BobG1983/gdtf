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

// The view lists of every MissingViews finding raised against the def this names.
fn missing_views(findings: &[ContentFinding], referrer_hint: &str) -> Vec<Vec<String>> {
    findings
        .iter()
        .filter_map(|finding| match finding {
            ContentFinding::MissingViews { referrer, views }
                if referrer.contains(referrer_hint) =>
            {
                Some(views.iter().map(|view| (**view).clone()).collect())
            }
            _ => None,
        })
        .collect()
}

// The referrer of the first dangling finding against this target, family and scheme.
fn dangling_referrer(
    findings: &[ContentFinding],
    target: &str,
    family: &str,
    scheme: ReferenceKeyScheme,
) -> Option<String> {
    findings.iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referrer,
            target: found_target,
            family: found_family,
            scheme: found_scheme,
        } if ***found_target == *target
            && ***found_family == *family
            && *found_scheme == scheme =>
        {
            Some((**referrer).clone())
        }
        _ => None,
    })
}

fn validated_fixture_report() -> Vec<ContentFinding> {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(ref_integrity_root())
        .starting_in(AppState::Load)
        .build();
    advance_until_resource_exists::<ContentValidationDone>(&mut app);
    findings_snapshot(&app)
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
        "the terrain def's dangling view→sprite-def edge must be reported; findings: {report:?}",
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

    advance_until_resource_exists::<ContentValidationDone>(&mut app);

    let report = findings_snapshot(&app);

    assert_situation_edges(&report);
    assert_roster_loadout_edges(&report);
    assert_item_edges(&report);
    assert_theme_and_prefab_edges(&report);

    advance_until(&mut app, |app| {
        matches!(app_state(app), AppState::Intro | AppState::Running)
    });
}

#[test]
fn a_def_short_of_the_views_it_owes_is_reported_once_naming_all_of_them() {
    let report = validated_fixture_report();

    let raised = missing_views(&report, "Sparse Views Cover");
    assert_eq!(
        raised.len(),
        1,
        "the def is short two views and must be reported ONCE naming both, not once per view; \
         findings: {report:?}",
    );
    let Some(views) = raised.first() else { return };
    for view in ["Facing(South)", "Facing(West)"] {
        assert!(
            views.iter().any(|named| named == view),
            "the one finding must name {view}, a facing the def draws no art for; \
             findings: {report:?}",
        );
    }
    assert_eq!(
        views.len(),
        2,
        "the finding names exactly the views the def is missing; findings: {report:?}",
    );
}

#[test]
fn a_leaves_behind_naming_no_terrain_def_is_reported_under_the_uuid_scheme() {
    let report = validated_fixture_report();

    assert!(
        has_dangling(
            &report,
            "Ghost Successor",
            "00000000-0000-0000-0000-130700000dea",
            "TerrainDefRegistry",
            ReferenceKeyScheme::Uuid,
        ),
        "a leaves_behind naming a successor def no registry holds must be reported against the \
         terrain registry under the UUID scheme; findings: {report:?}",
    );
}

#[test]
fn a_leaves_behind_naming_no_sprite_def_is_reported_under_the_file_stem_scheme() {
    let report = validated_fixture_report();

    assert!(
        has_dangling(
            &report,
            "Ghost Leftover",
            "ghost_leftover_graphic",
            "SpriteDefRegistry",
            ReferenceKeyScheme::FileStem,
        ),
        "a leaves_behind naming a sprite no registry holds must be reported against the sprite \
         registry under the file-stem scheme; findings: {report:?}",
    );
}

#[test]
fn a_view_naming_no_sprite_def_is_reported_against_that_view() {
    let report = validated_fixture_report();

    let referrer = dangling_referrer(
        &report,
        "ghost_view_graphic",
        "SpriteDefRegistry",
        ReferenceKeyScheme::FileStem,
    );
    assert!(
        referrer.is_some(),
        "the one view naming a sprite no registry holds must be reported against the sprite \
         registry under the file-stem scheme; findings: {report:?}",
    );
    let Some(referrer) = referrer else { return };
    assert!(
        referrer.contains("Ghost View Cover") && referrer.contains("Facing(West)"),
        "the referrer names the def AND the view the bad key was authored on, or an author \
         cannot tell which row to fix; got `{referrer}`",
    );
    assert!(
        missing_views(&report, "Ghost View Cover").is_empty(),
        "this def's view set is complete, so the finding above cannot be coming from it being \
         short of views; findings: {report:?}",
    );
}

#[test]
fn shipped_content_graph_validates_with_zero_findings() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<ContentValidationDone>(&mut app);
    let report = findings_snapshot(&app);
    assert!(
        report.is_empty(),
        "the shipped content graph must carry ZERO dangling references; findings: {report:?}",
    );
}
