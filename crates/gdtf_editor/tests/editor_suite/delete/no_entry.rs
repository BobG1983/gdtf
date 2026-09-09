//! A delete is offered only for a family the delete registry holds an entry for.

use gdtf_assets::{ContentMemberKey, FindingFamily};
use gdtf_battle_sim::weapon::WeaponRegistry;
use gdtf_editor::{
    DeleteOutcome, DeleteRefusal, DeleteRegistry, DeleteRequest, EditorMcpAssetsRoot,
};

use crate::{
    content_shared::{advance::advance_to_published, app::editor_app_with_asset_root},
    delete::{
        fixture::{FIXTURE_GUN, weapon_name, write_fixture_weapon},
        harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
    },
};

// The label every sprite def reference finding carries; no entry deletes one.
const SPRITE_FAMILY: &str = "SpriteDefRegistry";

#[test]
fn a_delete_for_a_family_with_no_entry_is_refused_and_takes_nothing_out() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_weapon(dir.path(), FIXTURE_GUN),
        "the fixture weapon write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    assert!(
        !app.world()
            .resource::<DeleteRegistry>()
            .handles(&FindingFamily::new(SPRITE_FAMILY.to_owned())),
        "the editor app must build a DeleteRegistry holding no entry for SpriteDefRegistry. \
         Sprite deletion is out of scope for this build",
    );

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(SPRITE_FAMILY.to_owned()),
        ContentMemberKey::new(FIXTURE_GUN.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    assert_eq!(
        outcome,
        Some(DeleteOutcome::Refused(DeleteRefusal::NoEntry)),
        "a request naming a family the DeleteRegistry holds no entry for must be refused with \
         NoEntry",
    );
    assert!(
        app.world()
            .resource::<WeaponRegistry>()
            .spec(&weapon_name(FIXTURE_GUN))
            .is_some(),
        "a refused delete must take nothing out of the registry",
    );
}
