//! the GANG mode's own SAVE path feeds the authoring-validation
use bevy::asset::AssetServer;
use gdtf_assets::{ContentFamily, ContentIntegrityReport};
use gdtf_battle_sim::weapon::WeaponName;
use gdtf_content_editor::{GangDraft, draft_to_roster, gang_file_name, write_gang_in};
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{advance_to_published, editor_app_with_asset_root, has_dangling_ref};

const REARM_GANG: &str = "rearm_gang";

const SAVED_DANGLING_WEAPON: &str = "saved_missing_weapon";

const RESAVED_DANGLING_WEAPON: &str = "resaved_missing_weapon";

#[test]
fn gang_save_reload_rearms_validation_with_the_saved_keys() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut draft = GangDraft::new_gang();
    draft.set_name(REARM_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = WeaponName::new(SAVED_DANGLING_WEAPON.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    let written = write_gang_in(dir.path(), &name, &roster);
    assert!(
        written.is_ok(),
        "the real gang write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            has_dangling_ref(report, "WeaponRegistry", SAVED_DANGLING_WEAPON),
            "the SAVED gang's dangling weapon key must surface at editor launch; report: {:?}",
            report.findings(),
        );
    }

    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = WeaponName::new(RESAVED_DANGLING_WEAPON.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    let rewritten = write_gang_in(dir.path(), &name, &roster);
    assert!(
        rewritten.is_ok(),
        "the re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    let saved_path = format!("{}/{}", GangsFamily::FOLDER, gang_file_name(&name));
    app.world().resource::<AssetServer>().reload(saved_path);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| {
                has_dangling_ref(report, "WeaponRegistry", RESAVED_DANGLING_WEAPON)
            })
    });
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "WeaponRegistry", SAVED_DANGLING_WEAPON),
        "the report must be RESET and re-checked on re-arm — the first save's superseded weapon \
         finding must not persist; report: {:?}",
        report.findings(),
    );
}
