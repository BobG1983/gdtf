//! the GANG and FIELD modes' own SAVE paths feed the authoring-validation
use bevy::asset::AssetServer;
use cobalt_test_utils::advance_until;
use gdtf_assets::{
    ContentFamily, ContentIntegrityReport, ContentValidationDone, ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    effects::fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, ImmuneArmorTypes,
    },
    weapon::{DamageType, WeaponName},
};
use gdtf_content_families::{FieldsFamily, GangsFamily, situation::LoadedSituation};
use gdtf_editor::{
    GangDraft, draft_to_roster, field_file_name, gang_file_name, write_field_in, write_gang_in,
};

use crate::{
    authoring_validation::harness::has_malformed,
    content_shared::{
        advance::advance_to_published, app::editor_app_with_asset_root, findings::has_dangling_ref,
    },
};

const REARM_GANG: &str = "rearm_gang";

const SAVED_DANGLING_WEAPON: &str = "saved_missing_weapon";

const RESAVED_DANGLING_WEAPON: &str = "resaved_missing_weapon";

const REARM_FIELD: &str = "rearm_field";

const MALFORMED_FIELD_STEM: &str = "broken_field";

// Only `damage` differs between the field the test saves and the one it re-saves.
fn field_def(damage: FieldDamage) -> FieldDef {
    FieldDef::new(
        damage,
        DamageType::Chem,
        ImmuneArmorTypes::default(),
        FieldDuration::Permanent,
    )
}

#[test]
fn gang_save_reload_rearms_validation_with_the_saved_keys() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut draft = GangDraft::new_gang();
    draft.set_name(REARM_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = Some(WeaponName::new(SAVED_DANGLING_WEAPON.to_owned()));
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
            has_dangling_ref(
                report,
                "WeaponRegistry",
                SAVED_DANGLING_WEAPON,
                ReferenceKeyScheme::FileStem,
            ),
            "the SAVED gang's dangling weapon key must surface at editor launch; report: {:?}",
            report.findings(),
        );
    }

    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = Some(WeaponName::new(RESAVED_DANGLING_WEAPON.to_owned()));
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
                has_dangling_ref(
                    report,
                    "WeaponRegistry",
                    RESAVED_DANGLING_WEAPON,
                    ReferenceKeyScheme::FileStem,
                )
            })
    });
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(
            report,
            "WeaponRegistry",
            SAVED_DANGLING_WEAPON,
            ReferenceKeyScheme::FileStem,
        ),
        "the report must be RESET and re-checked on re-arm — the first save's superseded weapon \
         finding must not persist; report: {:?}",
        report.findings(),
    );
}

#[test]
fn an_edit_to_the_loaded_situation_rearms_validation_the_way_a_registry_edit_does() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some(),
        "the report must have published before the situation is touched",
    );

    {
        let Some(mut loaded) = app.world_mut().get_resource_mut::<LoadedSituation>() else {
            return;
        };
        loaded.situation_mut().combatants.rosters.clear();
    }
    app.update();

    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_none(),
        "the loaded situation is a watched record, so writing it re-arms validation",
    );
}

#[test]
fn field_save_reload_rearms_validation_and_republishes_findings() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let key = FieldKey::new(REARM_FIELD.to_owned());
    let written = write_field_in(dir.path(), &key, &field_def(FieldDamage::new(3)));
    assert!(
        written.is_ok(),
        "the real field write must succeed: {:?}",
        written.as_ref().err(),
    );

    let malformed_path = dir.path().join(FieldsFamily::FOLDER).join(format!(
        "{MALFORMED_FIELD_STEM}.{}",
        FieldsFamily::EXTENSION
    ));
    let planted = std::fs::write(&malformed_path, "(this is not a FieldDef");
    assert!(
        planted.is_ok(),
        "planting the malformed sibling must succeed"
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            has_malformed(report, MALFORMED_FIELD_STEM),
            "the malformed field sibling must surface as a MalformedFile finding at launch; \
             report: {:?}",
            report.findings(),
        );
    }

    let edited_def = field_def(FieldDamage::new(9));
    let rewritten = write_field_in(dir.path(), &key, &edited_def);
    assert!(
        rewritten.is_ok(),
        "the field re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    let saved_path = format!("{}/{}", FieldsFamily::FOLDER, field_file_name(&key));
    app.world().resource::<AssetServer>().reload(saved_path);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<FieldDefRegistry>()
            .is_some_and(|registry| registry.def(&key) == Some(&edited_def))
    });

    // Wait for the re-armed pass to publish a report without the malformed finding.
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| !has_malformed(report, MALFORMED_FIELD_STEM))
    });
}
