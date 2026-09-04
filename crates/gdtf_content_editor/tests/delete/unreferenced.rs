//! An unreferenced record is removed cleanly, with no replacement asked for.

use gdtf_content_editor::{DeleteOutcome, weapon_save_path_in};

use crate::{
    fixture::{ORPHAN_GUN, weapon_name, write_fixture_weapon},
    harness::{
        OUTCOME_UPDATES, OfferAnswer, TERRAIN_FAMILY, WEAPON_FAMILY, editor_on, is_published,
        run_delete,
    },
    records::{DELETED_PIECE, file_snapshot, write_terrain_def},
};

#[test]
fn a_terrain_def_nothing_names_is_removed_with_no_offer_and_no_other_file_touched() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let Some(deleted) = write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE) else {
        return;
    };
    if !write_fixture_weapon(dir.path(), ORPHAN_GUN) {
        return;
    }
    let before: Vec<_> = file_snapshot(dir.path())
        .into_iter()
        .filter(|(path, _bytes)| *path != deleted)
        .collect();

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        TERRAIN_FAMILY,
        DELETED_PIECE,
        &OfferAnswer::Nothing,
    );

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "no theme, prefab, situation list or other def names it, so it is removed within \
         {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert!(
        !settled.offered,
        "the outcome is decided by the in-use check, not by the row's marking, so nothing is \
         asked for",
    );
    assert!(!deleted.exists(), "the removed record's file must be gone");
    assert_eq!(
        file_snapshot(dir.path()),
        before,
        "the delete touches only the record it names",
    );
}

#[test]
fn a_weapon_nothing_names_is_removed_with_no_offer() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_fixture_weapon(dir.path(), ORPHAN_GUN) {
        return;
    }

    let mut app = editor_on(dir.path());
    let settled = run_delete(&mut app, WEAPON_FAMILY, ORPHAN_GUN, &OfferAnswer::Nothing);

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "no gang member and no emplacement names it, so it is removed within {OUTCOME_UPDATES} \
         updates; published at the end: {}",
        is_published(&app),
    );
    assert!(
        !settled.offered,
        "an unreferenced weapon is deleted cleanly whatever its row says",
    );
    assert!(
        !weapon_save_path_in(dir.path(), &weapon_name(ORPHAN_GUN)).exists(),
        "the removed weapon's file must be gone",
    );
}
