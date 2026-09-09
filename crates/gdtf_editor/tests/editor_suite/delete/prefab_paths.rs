//! A prefab rewrite writes the file the prefab was read from, and creates no second one.

use gdtf_editor::DeleteOutcome;

use crate::delete::{
    harness::{
        OUTCOME_UPDATES, OfferAnswer, TERRAIN_FAMILY, editor_on, is_published, member_key,
        run_delete,
    },
    records::{
        DELETED_PIECE, DELETED_THEME, REPLACEMENT_PIECE, REPLACEMENT_THEME, file_snapshot,
        file_text, write_prefab, write_terrain_def, write_theme_def,
    },
};

// The stem both prefabs share; they key apart on their themes.
const SHARED_STEM: &str = "entry_room";

#[test]
fn both_prefabs_are_written_back_to_their_own_files_and_no_new_file_appears() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE).is_none()
        || write_terrain_def(dir.path(), "replacement_piece", REPLACEMENT_PIECE).is_none()
        || write_theme_def(
            dir.path(),
            "first_theme",
            DELETED_THEME,
            REPLACEMENT_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_none()
        || write_theme_def(
            dir.path(),
            "second_theme",
            REPLACEMENT_THEME,
            REPLACEMENT_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_none()
    {
        return;
    }
    let Some(first) = write_prefab(
        dir.path(),
        "one_folder_no_display_name_names",
        SHARED_STEM,
        DELETED_THEME,
        &[DELETED_PIECE],
    ) else {
        return;
    };
    let Some(second) = write_prefab(
        dir.path(),
        "another_folder_no_display_name_names",
        SHARED_STEM,
        REPLACEMENT_THEME,
        &[DELETED_PIECE],
    ) else {
        return;
    };
    let deleted_file = dir
        .path()
        .join("content/terrain/deleted_piece.terrain_def.ron");
    let expected: Vec<_> = file_snapshot(dir.path())
        .into_iter()
        .map(|(path, _bytes)| path)
        .filter(|path| *path != deleted_file)
        .collect();

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        TERRAIN_FAMILY,
        DELETED_PIECE,
        &OfferAnswer::Confirm(Some(member_key(REPLACEMENT_PIECE))),
    );
    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    for prefab in [&first, &second] {
        let text = file_text(prefab);
        assert!(
            text.contains(REPLACEMENT_PIECE) && !text.contains(DELETED_PIECE),
            "the prefab must be written back to the file it was read from: {text}",
        );
    }
    let after: Vec<_> = file_snapshot(dir.path())
        .into_iter()
        .map(|(path, _bytes)| path)
        .collect();
    assert_eq!(
        after, expected,
        "a path rebuilt from the theme's display name would land somewhere else, leaving a \
         second file behind",
    );
}
