//! On-death `LeaveField` keys are checked on the editor host too.

use gdtf_assets::{ContentIntegrityReport, ReferenceKeyScheme};

use crate::{
    authoring_validation::harness::{dangling_ref_record, editor_app_on_fixture_root},
    content_shared::{advance::advance_to_published, findings::has_dangling_ref},
};

// The field a fixture terrain def leaves on death, which the fixture root holds no def for.
const GHOST_DEATH_FIELD: &str = "ghost_death_field";

#[test]
fn a_dangling_on_death_field_ref_names_the_terrain_record_to_rewrite() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };

    assert!(
        has_dangling_ref(
            report,
            "FieldDefRegistry",
            GHOST_DEATH_FIELD,
            ReferenceKeyScheme::FileStem,
        ),
        "a terrain def's on-death field key is checked on the editor host, so \
         `{GHOST_DEATH_FIELD}` must come back as a FieldDefRegistry file-stem miss; report: {:?}",
        report.findings(),
    );

    let record = dangling_ref_record(
        report,
        "FieldDefRegistry",
        GHOST_DEATH_FIELD,
        ReferenceKeyScheme::FileStem,
    );
    let Some((family, key, field)) = record else {
        return;
    };
    assert_eq!(
        (family.as_str(), key.as_str(), field.as_str()),
        (
            "TerrainDefRegistry",
            "00000000-0000-0000-0000-130700000b05",
            "on_death[].LeaveField.field",
        ),
        "a drop must be able to name the terrain def to rewrite without parsing prose",
    );
}
