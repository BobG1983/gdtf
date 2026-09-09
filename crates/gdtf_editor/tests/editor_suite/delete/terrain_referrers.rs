//! Deleting a terrain piece points every kind of referrer at the replacement instead.

use std::path::{Path, PathBuf};

use bevy::app::App;
use gdtf_editor::DeleteOutcome;
use tempfile::TempDir;

use crate::delete::{
    harness::{
        OUTCOME_UPDATES, OfferAnswer, TERRAIN_FAMILY, editor_on, is_published, member_key,
        run_delete,
    },
    records::{
        DELETED_PIECE, REPLACEMENT_PIECE, file_text, root_holds, write_leaves_behind_def,
        write_prefab, write_situation, write_terrain_def, write_theme_def,
    },
};

// The two pieces every case in this file holds, one deleted and one replacing it.
fn two_pieces(root: &Path) -> bool {
    write_terrain_def(root, "deleted_piece", DELETED_PIECE).is_some()
        && write_terrain_def(root, "replacement_piece", REPLACEMENT_PIECE).is_some()
}

// A temp root with both pieces written, or nothing when the writes failed.
fn root_with_pieces() -> Option<TempDir> {
    let dir = tempfile::tempdir().ok()?;
    two_pieces(dir.path()).then_some(dir)
}

// Delete the piece with the replacement chosen, asserting the delete settled as Removed.
fn delete_with_replacement(app: &mut App) {
    let settled = run_delete(
        app,
        TERRAIN_FAMILY,
        DELETED_PIECE,
        &OfferAnswer::Confirm(Some(member_key(REPLACEMENT_PIECE))),
    );
    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the delete must settle as Removed within {OUTCOME_UPDATES} updates; published at the \
         end: {}",
        is_published(app),
    );
}

// The referring file holds the replacement, and no file under the root holds the deleted key.
fn assert_repointed(root: &Path, referrer: &Path) {
    let text = file_text(referrer);
    assert!(
        text.contains(REPLACEMENT_PIECE),
        "the referring file must name the replacement piece after the rewrite: {text}",
    );
    assert!(
        !root_holds(root, DELETED_PIECE),
        "no file under the root may still name the deleted piece",
    );
}

#[test]
fn deleting_a_piece_repoints_a_theme_default_floor() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(theme) = write_theme_def(
        dir.path(),
        "floor_theme",
        crate::delete::records::DELETED_THEME,
        DELETED_PIECE,
        &[REPLACEMENT_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    assert_repointed(dir.path(), &theme);
}

#[test]
fn deleting_a_piece_repoints_a_theme_terrain_palette() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(theme) = write_theme_def(
        dir.path(),
        "palette_theme",
        crate::delete::records::DELETED_THEME,
        REPLACEMENT_PIECE,
        &[DELETED_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    assert_repointed(dir.path(), &theme);
}

#[test]
fn deleting_a_piece_repoints_a_prefab_placement() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(theme) = theme_holding_both(dir.path()) else {
        return;
    };
    let Some(prefab) = write_prefab(
        dir.path(),
        "some_folder",
        "entry_room",
        crate::delete::records::DELETED_THEME,
        &[DELETED_PIECE],
    ) else {
        return;
    };
    drop(theme);

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    assert_repointed(dir.path(), &prefab);
}

#[test]
fn deleting_a_piece_repoints_the_situations_piece_lists_and_default_floor() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(situation) = write_situation(
        dir.path(),
        &format!(
            "(
    default_floor: \"{DELETED_PIECE}\",
    walls: [(at: (cell: (x: 1, y: 1), level: 0), piece: \"{DELETED_PIECE}\")],
    scatter: [(at: (cell: (x: 2, y: 1), level: 0), piece: \"{DELETED_PIECE}\")],
    slabs: [(at: (cell: (x: 3, y: 1), level: 0), piece: \"{DELETED_PIECE}\")],
    floors: [(at: (cell: (x: 4, y: 1), level: 0), piece: \"{DELETED_PIECE}\")],
)
"
        ),
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    assert_repointed(dir.path(), &situation);
}

#[test]
fn deleting_a_piece_repoints_another_defs_leaves_behind() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(successor) = write_leaves_behind_def(
        dir.path(),
        "successor_def",
        "00000000-0000-0000-0000-133000000b01",
        DELETED_PIECE,
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    assert_repointed(dir.path(), &successor);
}

#[test]
fn every_occurrence_in_one_record_is_repointed_not_only_the_one_a_finding_names() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(situation) = write_situation(
        dir.path(),
        &format!(
            "(
    walls: [(at: (cell: (x: 1, y: 1), level: 0), piece: \"{DELETED_PIECE}\")],
    floors: [(at: (cell: (x: 2, y: 2), level: 0), piece: \"{DELETED_PIECE}\")],
)
"
        ),
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    let text = file_text(&situation);
    assert_eq!(
        text.matches(REPLACEMENT_PIECE).count(),
        2,
        "the situation names the deleted piece in walls AND in floors, and the check raises one \
         finding for the pair, so both lists must be repointed: {text}",
    );
    assert!(
        !text.contains(DELETED_PIECE),
        "no list may keep the deleted piece: {text}",
    );
}

#[test]
fn a_palette_that_already_holds_the_replacement_holds_it_once_after_the_rewrite() {
    let Some(dir) = root_with_pieces() else {
        return;
    };
    let Some(theme) = write_theme_def(
        dir.path(),
        "both_theme",
        crate::delete::records::DELETED_THEME,
        REPLACEMENT_PIECE,
        &[DELETED_PIECE, REPLACEMENT_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_with_replacement(&mut app);
    let text = file_text(&theme);
    let palette = text
        .split_once("terrain:")
        .map_or_else(String::new, |(_head, tail)| tail.to_owned());
    assert_eq!(
        palette.matches(REPLACEMENT_PIECE).count(),
        1,
        "the palette already held the replacement, so mapping the deleted key onto it must not \
         list it twice: {text}",
    );
}

// A theme holding the replacement, so a prefab naming it raises no dangling theme edge.
fn theme_holding_both(root: &Path) -> Option<PathBuf> {
    write_theme_def(
        root,
        "prefab_theme",
        crate::delete::records::DELETED_THEME,
        REPLACEMENT_PIECE,
        &[REPLACEMENT_PIECE],
    )
}
