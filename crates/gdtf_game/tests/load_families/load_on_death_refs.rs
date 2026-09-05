//! On-death `LeaveField` keys are checked against the field catalog at load.
use std::path::PathBuf;

use cobalt_test_utils::{LoadTestAppBuilder, advance_until_resource_exists};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentValidationDone, ReferenceKeyScheme,
};
use gdtf_game::test_support::AppState;

fn ref_integrity_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ref_integrity_root")
}

fn validated_fixture_report() -> Vec<ContentFinding> {
    let mut app = LoadTestAppBuilder::with_asset_root(
        ref_integrity_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();
    advance_until_resource_exists::<ContentValidationDone>(&mut app);
    app.world()
        .get_resource::<ContentIntegrityReport>()
        .map(|report| report.findings().to_vec())
        .unwrap_or_default()
}

// The referring record of the first dangling field finding against this target.
fn field_ref_record(findings: &[ContentFinding], target: &str) -> Option<(String, String, String)> {
    findings.iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referring_record,
            target: found_target,
            family,
            scheme,
            ..
        } if ***found_target == *target
            && ***family == *"FieldDefRegistry"
            && *scheme == ReferenceKeyScheme::FileStem =>
        {
            Some((
                (*referring_record.family).clone(),
                (*referring_record.key).clone(),
                (*referring_record.field).clone(),
            ))
        }
        _ => None,
    })
}

#[test]
fn a_terrain_on_death_naming_no_field_def_names_the_terrain_record_to_rewrite() {
    let report = validated_fixture_report();

    let record = field_ref_record(&report, "ghost_death_field");
    assert!(
        record.is_some(),
        "a terrain def leaving a field no catalog holds must be reported against the field \
         registry under the file-stem scheme; findings: {report:?}",
    );
    let Some((family, member, field)) = record else {
        return;
    };
    assert_eq!(
        (family.as_str(), member.as_str(), field.as_str()),
        (
            "TerrainDefRegistry",
            "00000000-0000-0000-0000-130700000a05",
            "on_death[].LeaveField.field",
        ),
        "the finding names the terrain def a drop would rewrite, keyed by its UUID",
    );
}

#[test]
fn a_weapon_on_death_naming_no_field_def_names_the_weapon_record_to_rewrite() {
    let report = validated_fixture_report();

    let record = field_ref_record(&report, "ghost_gun_field");
    assert!(
        record.is_some(),
        "a ranged weapon spec leaving a field no catalog holds must be reported against the \
         field registry under the file-stem scheme; findings: {report:?}",
    );
    let Some((family, member, field)) = record else {
        return;
    };
    assert_eq!(
        (family.as_str(), member.as_str(), field.as_str()),
        (
            "WeaponRegistry",
            "fixture_gun",
            "on_death[].LeaveField.field",
        ),
        "the finding names the weapon spec a drop would rewrite, keyed by its file stem",
    );
}
