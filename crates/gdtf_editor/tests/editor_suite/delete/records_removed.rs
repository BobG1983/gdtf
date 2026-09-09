//! The file a delete removes is the one the record was read from, not one rebuilt by name.

use gdtf_battle_sim::{
    ganger::{GangName, GangRegistry},
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_editor::{DeleteOutcome, GangDraft, draft_to_roster, write_gang_in};

use crate::delete::{
    harness::{
        GANG_FAMILY, OUTCOME_UPDATES, OfferAnswer, TERRAIN_FAMILY, THEME_FAMILY, editor_on,
        is_published, member_key, run_delete,
    },
    records::{
        DELETED_PIECE, DELETED_THEME, REPLACEMENT_PIECE, write_situation, write_terrain_def,
        write_theme_def,
    },
};

// The gang the situation names, and the one that replaces it.
const DELETED_GANG: &str = "deleted_gang";

// The replacement gang, whose roster holds the same member.
const MATCHING_GANG: &str = "matching_gang";

// The one member the situation asks both gangs for.
const MEMBER: &str = "Scrap";

#[test]
fn deleting_a_terrain_def_removes_the_file_it_was_read_from() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let Some(file) = write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE) else {
        return;
    };

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
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert!(
        !file.exists(),
        "the def's display name does not match its file stem, so a path rebuilt from the name \
         would miss the file the loader read",
    );
    assert!(
        terrain_uuid().is_some_and(|uuid| app
            .world()
            .resource::<TerrainDefRegistry>()
            .def(&uuid)
            .is_none()),
        "a re-read registry must no longer hold the deleted key",
    );
}

#[test]
fn deleting_a_theme_def_removes_the_file_it_was_read_from() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if write_terrain_def(dir.path(), "kept_piece", REPLACEMENT_PIECE).is_none() {
        return;
    }
    let Some(file) = write_theme_def(
        dir.path(),
        "deleted_theme",
        DELETED_THEME,
        REPLACEMENT_PIECE,
        &[REPLACEMENT_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    let settled = run_delete(&mut app, THEME_FAMILY, DELETED_THEME, &OfferAnswer::Nothing);

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert!(
        !file.exists(),
        "the theme's display name does not match its file stem, so only the recorded file is \
         the right one to remove",
    );
    assert!(
        theme_uuid().is_some_and(|uuid| app
            .world()
            .resource::<UuidThemeRegistry>()
            .def(&uuid)
            .is_none()),
        "a re-read registry must no longer hold the deleted theme",
    );
}

#[test]
fn deleting_a_gang_removes_its_own_file_and_takes_it_out_of_the_registry() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !write_gang(dir.path(), DELETED_GANG) || !write_gang(dir.path(), MATCHING_GANG) {
        return;
    }
    let file = gdtf_editor::gang_save_path_in(dir.path(), &GangName::new(DELETED_GANG.to_owned()));
    if write_situation(
        dir.path(),
        &format!(
            "(combatants: (rosters: [(gang: \"{DELETED_GANG}\", member: \"{MEMBER}\", faction: \
             0)]))\n"
        ),
    )
    .is_none()
    {
        return;
    }

    let mut app = editor_on(dir.path());
    let settled = run_delete(
        &mut app,
        GANG_FAMILY,
        DELETED_GANG,
        &OfferAnswer::Confirm(Some(member_key(MATCHING_GANG))),
    );

    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert!(!file.exists(), "the deleted gang's own file must be gone");
    assert!(
        app.world()
            .resource::<GangRegistry>()
            .roster(&GangName::new(DELETED_GANG.to_owned()))
            .is_none(),
        "the deleted gang must stay out of GangRegistry",
    );
}

// A one-member gang at the top of the gangs folder, where the rewrite writes it back.
fn write_gang(root: &std::path::Path, stem: &str) -> bool {
    let mut draft = GangDraft::new_gang();
    draft.set_name(stem.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.name = gdtf_battle_sim::ganger::GangerName::new(MEMBER.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    write_gang_in(root, &name, &roster).is_ok()
}

// The deleted piece's key, as a terrain UUID.
fn terrain_uuid() -> Option<TerrainUuid> {
    bevy::asset::uuid::Uuid::parse_str(DELETED_PIECE)
        .ok()
        .map(TerrainUuid::new)
}

// The deleted theme's key, as a theme UUID.
fn theme_uuid() -> Option<ThemeUuid> {
    bevy::asset::uuid::Uuid::parse_str(DELETED_THEME)
        .ok()
        .map(ThemeUuid::new)
}
