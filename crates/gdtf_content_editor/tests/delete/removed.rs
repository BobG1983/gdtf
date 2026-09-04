//! A delete whose in-use check finds no referring record removes the record.

use gdtf_assets::{ContentMemberKey, FindingFamily};
use gdtf_battle_sim::weapon::WeaponRegistry;
use gdtf_content_editor::{DeleteOutcome, DeleteRequest, EditorQaAssetsRoot, weapon_save_path_in};

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    fixture::{FIXTURE_GUN, ORPHAN_GUN, weapon_name, write_fixture_gang, write_fixture_weapon},
    harness::{OUTCOME_UPDATES, WEAPON_FAMILY, advance_to_outcome, is_published},
};

#[test]
fn a_weapon_no_record_names_is_removed_with_its_file() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_weapon(dir.path(), FIXTURE_GUN),
        "the fixture weapon write must succeed",
    );
    assert!(
        write_fixture_weapon(dir.path(), ORPHAN_GUN),
        "the orphan weapon write must succeed",
    );
    assert!(
        write_fixture_gang(dir.path(), FIXTURE_GUN),
        "the fixture gang write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorQaAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(WEAPON_FAMILY.to_owned()),
        ContentMemberKey::new(ORPHAN_GUN.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "a delete no record references must settle as Removed",
    );
    assert!(
        app.world()
            .resource::<WeaponRegistry>()
            .spec(&weapon_name(ORPHAN_GUN))
            .is_none(),
        "the removed record must stay out of its registry",
    );
    assert!(
        !weapon_save_path_in(dir.path(), &weapon_name(ORPHAN_GUN)).exists(),
        "the removed record's file must be gone — otherwise the next folder reload brings it \
         back",
    );
    assert!(
        weapon_save_path_in(dir.path(), &weapon_name(FIXTURE_GUN)).exists(),
        "the delete must touch only the record it names",
    );
}
