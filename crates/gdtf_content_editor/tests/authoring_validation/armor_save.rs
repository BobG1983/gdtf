//! the ARMOR mode's own SAVE path feeds the authoring-validation
use bevy::asset::AssetServer;
use gdtf_assets::{ContentFamily, ContentIntegrityReport, ReferenceKeyScheme};
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorProtection, ArmorRegistry, BodyPart},
    weapon::WeaponName,
};
use gdtf_content_editor::{
    ArmorDraft, GangDraft, armor_file_name, draft_to_roster, draft_to_spec, write_armor_in,
    write_gang_in,
};
use gdtf_content_families::ArmorFamily;
use gdtf_test_utils::advance_until;

use crate::{
    advance::advance_to_published, app::editor_app_with_asset_root, findings::has_dangling_ref,
    harness::has_malformed,
};

const REARM_ARMOR: &str = "rearm_plate";

const DANGLING_WEAPON: &str = "armor_suite_missing_weapon";

const MALFORMED_STEM: &str = "broken_plate";

// What the report must say the first time the editor reads the planted root.
fn assert_launch_findings(report: &ContentIntegrityReport) {
    assert!(
        !has_dangling_ref(
            report,
            "ArmorRegistry",
            REARM_ARMOR,
            ReferenceKeyScheme::FileStem,
        ),
        "the SAVED armor must resolve the gang's armor key at editor launch (no dangling \
         ArmorRegistry finding); report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_ref(
            report,
            "WeaponRegistry",
            DANGLING_WEAPON,
            ReferenceKeyScheme::FileStem,
        ),
        "the gang's dangling weapon key must surface at editor launch; report: {:?}",
        report.findings(),
    );
    assert!(
        has_malformed(report, MALFORMED_STEM),
        "the malformed armor sibling must surface as a MalformedFile finding at launch; \
         report: {:?}",
        report.findings(),
    );
}

#[test]
fn armor_save_reload_rearms_validation_and_republishes_findings() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut armor_draft = ArmorDraft::new_armor();
    armor_draft.set_name(REARM_ARMOR.to_owned());
    armor_draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(4);
    let (armor_name, armor_spec) = draft_to_spec(&armor_draft);
    let written = write_armor_in(dir.path(), &armor_name, &armor_spec);
    assert!(
        written.is_ok(),
        "the real armor write must succeed: {:?}",
        written.as_ref().err(),
    );

    let malformed_path = dir
        .path()
        .join(ArmorFamily::FOLDER)
        .join(format!("{MALFORMED_STEM}.{}", ArmorFamily::EXTENSION));
    let planted = std::fs::write(&malformed_path, "(this is not an ArmorSpec");
    assert!(
        planted.is_ok(),
        "planting the malformed sibling must succeed"
    );

    let mut gang_draft = GangDraft::new_gang();
    gang_draft.set_name("armor_rearm_gang".to_owned());
    gang_draft.add_member();
    if let Some(member) = gang_draft.members_mut().first_mut() {
        member.armor = Some(ArmorName::new(REARM_ARMOR.to_owned()));
        member.weapon = Some(WeaponName::new(DANGLING_WEAPON.to_owned()));
    }
    let (gang_name, roster) = draft_to_roster(&gang_draft);
    let gang_written = write_gang_in(dir.path(), &gang_name, &roster);
    assert!(
        gang_written.is_ok(),
        "the real gang write must succeed: {:?}",
        gang_written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    assert_launch_findings(app.world().resource::<ContentIntegrityReport>());

    armor_draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(9);
    let (armor_name, edited_spec) = draft_to_spec(&armor_draft);
    let rewritten = write_armor_in(dir.path(), &armor_name, &edited_spec);
    assert!(
        rewritten.is_ok(),
        "the armor re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    let saved_path = format!("{}/{}", ArmorFamily::FOLDER, armor_file_name(&armor_name));
    app.world().resource::<AssetServer>().reload(saved_path);

    advance_until(&mut app, |app| {
        let registry_rebuilt =
            app.world()
                .get_resource::<ArmorRegistry>()
                .is_some_and(|registry| {
                    registry.spec(&ArmorName::new(REARM_ARMOR.to_owned())) == Some(&edited_spec)
                });
        let report_fresh = app
            .world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| {
                has_dangling_ref(
                    report,
                    "WeaponRegistry",
                    DANGLING_WEAPON,
                    ReferenceKeyScheme::FileStem,
                ) && !has_malformed(report, MALFORMED_STEM)
            });
        registry_rebuilt && report_fresh
    });
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(
            report,
            "ArmorRegistry",
            REARM_ARMOR,
            ReferenceKeyScheme::FileStem,
        ),
        "the re-saved armor key must still resolve after the re-check; report: {:?}",
        report.findings(),
    );
}
