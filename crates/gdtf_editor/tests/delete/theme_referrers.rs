//! Deleting a theme points every prefab and the situation at the replacement theme.

use std::path::Path;

use bevy::{app::App, asset::uuid::Uuid};
use gdtf_assets::{ContentFileStem, ContentMemberKey, ContentSourcePaths};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabSpec, SpawnRole, ThemeUuid,
};
use gdtf_content_families::{PrefabsFamily, prefabs::member_key as prefab_member_key};
use gdtf_editor::DeleteOutcome;

use crate::{
    harness::{
        OUTCOME_UPDATES, OfferAnswer, THEME_FAMILY, editor_on, is_published, member_key, run_delete,
    },
    records::{
        DELETED_THEME, REPLACEMENT_PIECE, REPLACEMENT_THEME, file_text, write_prefab,
        write_situation, write_terrain_def, write_theme_def,
    },
};

// The prefab both theme cases author, under a folder no theme display name names.
const PREFAB_STEM: &str = "entry_room";

// Both themes, each holding the one piece the root defines.
fn two_themes(root: &Path) -> bool {
    write_terrain_def(root, "kept_piece", REPLACEMENT_PIECE).is_some()
        && write_theme_def(
            root,
            "deleted_theme",
            DELETED_THEME,
            REPLACEMENT_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_some()
        && write_theme_def(
            root,
            "replacement_theme",
            REPLACEMENT_THEME,
            REPLACEMENT_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_some()
}

// Delete the theme with the replacement chosen, asserting the delete settled as Removed.
fn delete_theme(app: &mut App) {
    let settled = run_delete(
        app,
        THEME_FAMILY,
        DELETED_THEME,
        &OfferAnswer::Confirm(Some(member_key(REPLACEMENT_THEME))),
    );
    assert_eq!(
        settled.outcome,
        Some(DeleteOutcome::Removed),
        "the theme delete must settle as Removed within {OUTCOME_UPDATES} updates; published at \
         the end: {}",
        is_published(app),
    );
}

// The key a prefab of `theme` is recorded under.
fn prefab_key(theme: &str) -> ContentMemberKey {
    let uuid = Uuid::parse_str(theme).map_or_else(|_error| ThemeUuid::nil(), ThemeUuid::new);
    let size = GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1))
        .unwrap_or_default();
    prefab_member_key(
        &ContentFileStem::new(PREFAB_STEM.to_owned()),
        &PrefabSpec::new(uuid, size, SpawnRole::Fill, Vec::new()),
    )
}

#[test]
fn deleting_a_theme_repoints_a_prefabs_theme_and_follows_its_recorded_file() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !two_themes(dir.path()) {
        return;
    }
    let Some(prefab) = write_prefab(
        dir.path(),
        "a_folder_no_display_name_names",
        PREFAB_STEM,
        DELETED_THEME,
        &[REPLACEMENT_PIECE],
    ) else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_theme(&mut app);

    let text = file_text(&prefab);
    assert!(
        text.contains(REPLACEMENT_THEME) && !text.contains(DELETED_THEME),
        "the prefab's own file must name the replacement theme and not the deleted one: {text}",
    );
    assert!(
        recorded_file(&app, &prefab_key(REPLACEMENT_THEME))
            .is_some_and(|relative| dir.path().join(relative) == prefab),
        "the rewritten prefab's recorded file must be the one it was read from, under the key \
         its rewritten spec builds",
    );
    assert!(
        recorded_file(&app, &prefab_key(DELETED_THEME)).is_none(),
        "the entry must move off the key the prefab was held at, or the next save writes a \
         rebuilt path",
    );
}

#[test]
fn deleting_a_theme_repoints_the_situations_own_theme() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if !two_themes(dir.path()) {
        return;
    }
    let Some(situation) = write_situation(dir.path(), &format!("(theme: \"{DELETED_THEME}\")\n"))
    else {
        return;
    };

    let mut app = editor_on(dir.path());
    delete_theme(&mut app);

    let text = file_text(&situation);
    assert!(
        text.contains(REPLACEMENT_THEME) && !text.contains(DELETED_THEME),
        "the situation must name the replacement theme and not the deleted one: {text}",
    );
}

// The file the prefab family recorded for one key, as the editor's load pass built it.
fn recorded_file(app: &App, key: &ContentMemberKey) -> Option<std::path::PathBuf> {
    app.world()
        .get_resource::<ContentSourcePaths<PrefabsFamily>>()?
        .path(key)
        .map(|relative| (**relative).clone())
}
