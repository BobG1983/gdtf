//! Cancelling a delete writes nothing, removes nothing, and puts the record back.

use gdtf_battle_sim::{
    terrain::def::{TerrainDefRegistry, TerrainUuid},
    weapon::WeaponRegistry,
};
use gdtf_editor::DeleteOutcome;

use crate::delete::{
    fixture::{FIXTURE_GUN, ORPHAN_GUN, weapon_name, write_fixture_gang, write_fixture_weapon},
    harness::{OfferAnswer, TERRAIN_FAMILY, WEAPON_FAMILY, editor_on, is_published, run_delete},
    records::{
        DELETED_PIECE, DELETED_THEME, REPLACEMENT_PIECE, file_snapshot, write_emplacement_def,
        write_leaves_behind_def, write_prefab, write_situation, write_terrain_def, write_theme_def,
    },
};

// The emplacement terrain def the weapon case mounts the deleted weapon on.
const MOUNTING_DEF: &str = "00000000-0000-0000-0000-133000000e03";

// The def whose `leaves_behind` names the deleted piece.
const SUCCESSOR_DEF: &str = "00000000-0000-0000-0000-133000000b02";

#[test]
fn cancelling_a_terrain_delete_leaves_every_file_and_the_registry_as_they_were() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE).is_none()
        || write_terrain_def(dir.path(), "replacement_piece", REPLACEMENT_PIECE).is_none()
        || write_leaves_behind_def(dir.path(), "successor_def", SUCCESSOR_DEF, DELETED_PIECE)
            .is_none()
        || write_theme_def(
            dir.path(),
            "floor_theme",
            DELETED_THEME,
            DELETED_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_none()
        || write_prefab(
            dir.path(),
            "a_folder",
            "entry_room",
            DELETED_THEME,
            &[DELETED_PIECE],
        )
        .is_none()
        || write_situation(
            dir.path(),
            &format!(
                "(map: (walls: [(at: (cell: (x: 1, y: 1), level: 0), piece: \
                 \"{DELETED_PIECE}\")]))\n"
            ),
        )
        .is_none()
    {
        return;
    }
    let before = file_snapshot(dir.path());

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        TERRAIN_FAMILY,
        DELETED_PIECE,
        &OfferAnswer::Cancel,
    );

    assert_eq!(
        settled.outcome,
        DeleteOutcome::Cancelled,
        "a cancel settles as Cancelled, which is not the same answer as a refusal; published at \
         the end: {}",
        is_published(&app),
    );
    assert_eq!(
        file_snapshot(dir.path()),
        before,
        "a cancel writes nothing and removes nothing, so every file under the root is unchanged",
    );
    assert!(
        deleted_piece_uuid().is_some_and(|uuid| app
            .world()
            .resource::<TerrainDefRegistry>()
            .def(&uuid)
            .is_some()),
        "a cancelled delete must put the record back under its own key",
    );
}

#[test]
fn cancelling_a_weapon_delete_writes_neither_the_gang_nor_the_emplacement() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_fixture_weapon(dir.path(), FIXTURE_GUN)
        || !write_fixture_weapon(dir.path(), ORPHAN_GUN)
        || !write_fixture_gang(dir.path(), FIXTURE_GUN)
        || write_emplacement_def(dir.path(), "mounted_def", MOUNTING_DEF, FIXTURE_GUN).is_none()
    {
        return;
    }
    let before = file_snapshot(dir.path());

    let mut app = editor_on(dir.path());
    let settled = run_delete(&mut app, WEAPON_FAMILY, FIXTURE_GUN, &OfferAnswer::Cancel);

    assert_eq!(
        settled.outcome,
        DeleteOutcome::Cancelled,
        "a cancel settles as Cancelled; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        file_snapshot(dir.path()),
        before,
        "the gang's optional reference is dropped only on a confirm, so a cancel leaves the gang \
         file and the emplacement def exactly as they were",
    );
    assert!(
        app.world()
            .resource::<WeaponRegistry>()
            .spec(&weapon_name(FIXTURE_GUN))
            .is_some(),
        "a cancelled delete must put the weapon back into its registry",
    );
}

// The deleted piece's key, as a terrain UUID.
fn deleted_piece_uuid() -> Option<TerrainUuid> {
    bevy::asset::uuid::Uuid::parse_str(DELETED_PIECE)
        .ok()
        .map(TerrainUuid::new)
}
