//! Confirming with nothing chosen refuses, naming the record that still holds the reference.

use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_editor::{DeleteOutcome, DeleteRefusal};

use crate::{
    harness::{OUTCOME_UPDATES, OfferAnswer, TERRAIN_FAMILY, editor_on, is_published, run_delete},
    records::{
        DELETED_PIECE, DELETED_THEME, REPLACEMENT_PIECE, file_text, write_terrain_def,
        write_theme_def,
    },
};

#[test]
fn a_confirm_with_no_replacement_chosen_refuses_and_leaves_the_reference_standing() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let Some(deleted) = write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE) else {
        return;
    };
    if write_terrain_def(dir.path(), "replacement_piece", REPLACEMENT_PIECE).is_none() {
        return;
    }
    let Some(theme) = write_theme_def(
        dir.path(),
        "floor_theme",
        DELETED_THEME,
        DELETED_PIECE,
        &[REPLACEMENT_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        TERRAIN_FAMILY,
        DELETED_PIECE,
        &OfferAnswer::Confirm(None),
    );

    let referring = match settled.outcome {
        Some(DeleteOutcome::Refused(DeleteRefusal::InUse(ref records))) => records.clone(),
        _ => Vec::new(),
    };
    assert!(
        referring.iter().any(|record| *record.key == *DELETED_THEME),
        "the refusal must name the theme that still holds the reference; settled as {:?} within \
         {OUTCOME_UPDATES} updates, published at the end: {}",
        settled.outcome,
        is_published(&app),
    );
    assert!(
        file_text(&theme).contains(DELETED_PIECE),
        "a refused delete writes no referrer, so the theme still names the piece",
    );
    assert!(
        deleted.exists(),
        "a refused delete must leave the record's own file on disk",
    );
    assert!(
        deleted_piece_uuid().is_some_and(|uuid| app
            .world()
            .resource::<TerrainDefRegistry>()
            .def(&uuid)
            .is_some()),
        "a refused delete must put the record back into its registry",
    );
}

// The deleted piece's key, as a terrain UUID.
fn deleted_piece_uuid() -> Option<TerrainUuid> {
    bevy::asset::uuid::Uuid::parse_str(DELETED_PIECE)
        .ok()
        .map(TerrainUuid::new)
}
