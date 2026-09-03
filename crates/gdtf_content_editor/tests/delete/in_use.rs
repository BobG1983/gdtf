//! No delete completes while a reference to that record is unresolved.

use bevy::prelude::ResMut;
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, ContentValidationAppExt as _,
    FindingFamily, FindingReferrer, FindingTarget, ReferenceField, ReferenceKeyScheme,
    ReferringRecord,
};
use gdtf_battle_sim::weapon::WeaponRegistry;
use gdtf_content_editor::{
    DeleteOutcome, DeleteRefusal, DeleteRegistry, DeleteRequest, EditorQaAssetsRoot,
    weapon_save_path_in,
};

use crate::{
    fixture::{FIXTURE_GANG, FIXTURE_GUN, weapon_name, write_fixture_gang, write_fixture_weapon},
    harness::{
        OUTCOME_UPDATES, WEAPON_FAMILY, advance_to_outcome, advance_to_published,
        editor_app_with_asset_root, is_published, weapon_delete_entry,
    },
};

// A second referrer for the fixture weapon, from a family the gang check never walks.
const EXTRA_REFERRER: &str = "fixture_emplacement";

// Records one more dangling reference to the fixture weapon on every Check pass.
fn record_extra_weapon_ref(mut report: ResMut<ContentIntegrityReport>) {
    report.record(ContentFinding::DanglingRef {
        referrer:         FindingReferrer::new(format!(
            "terrain def `{EXTRA_REFERRER}` emplacement mounted_weapon"
        )),
        referring_record: ReferringRecord::new(
            FindingFamily::new("TerrainDefRegistry".to_owned()),
            ContentMemberKey::new(EXTRA_REFERRER.to_owned()),
            ReferenceField::new("sim_kind.mounted_weapon".to_owned()),
        ),
        target:           FindingTarget::new(FIXTURE_GUN.to_owned()),
        family:           FindingFamily::new(WEAPON_FAMILY.to_owned()),
        scheme:           ReferenceKeyScheme::FileStem,
    });
}

#[test]
fn the_in_use_check_answers_with_every_referring_record_the_report_holds() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_weapon(dir.path(), FIXTURE_GUN),
        "the fixture weapon write must succeed",
    );
    assert!(
        write_fixture_gang(dir.path(), FIXTURE_GUN),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorQaAssetsRoot::new(dir.path().to_path_buf()));
    app.register_reference_check(record_extra_weapon_ref);
    app.world_mut()
        .resource_mut::<DeleteRegistry>()
        .add(weapon_delete_entry());
    advance_to_published(&mut app);

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(WEAPON_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_GUN.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    let referring = match outcome {
        Some(DeleteOutcome::Refused(DeleteRefusal::InUse(ref records))) => records.clone(),
        _ => Vec::new(),
    };
    assert_eq!(
        referring.len(),
        2,
        "the in-use check must read the WHOLE Check set output — the gang's reference AND the \
         test's registered check must both come back; outcome: {outcome:?}",
    );
}

#[test]
fn a_weapon_a_gang_member_still_names_is_not_deleted() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_weapon(dir.path(), FIXTURE_GUN),
        "the fixture weapon write must succeed",
    );
    assert!(
        write_fixture_gang(dir.path(), FIXTURE_GUN),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorQaAssetsRoot::new(dir.path().to_path_buf()));
    app.world_mut()
        .resource_mut::<DeleteRegistry>()
        .add(weapon_delete_entry());
    advance_to_published(&mut app);

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(WEAPON_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_GUN.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    let referring = match outcome {
        Some(DeleteOutcome::Refused(DeleteRefusal::InUse(ref records))) => records.clone(),
        _ => Vec::new(),
    };
    assert_eq!(
        referring.len(),
        1,
        "the refusal must name exactly the one record still referencing the weapon; \
         outcome: {outcome:?}",
    );
    assert!(
        referring
            .first()
            .is_some_and(|record| *record.key == *FIXTURE_GANG),
        "the referring record must be keyed `{FIXTURE_GANG}`; found: {referring:?}",
    );
    assert!(
        app.world()
            .resource::<WeaponRegistry>()
            .spec(&weapon_name(FIXTURE_GUN))
            .is_some(),
        "a refused delete must put the record back into its registry",
    );
    assert!(
        weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_GUN)).exists(),
        "a refused delete must leave the record's file on disk",
    );
}
