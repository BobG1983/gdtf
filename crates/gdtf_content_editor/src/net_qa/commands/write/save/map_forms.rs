//! Write the three map-authoring drafts, each the way its own save button does.

use std::path::Path;

use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainDefRegistry};

use crate::{
    editor_map::EditorMap,
    net_qa::wire::EditorContentNameNet,
    save,
    save_record::SaveOutcome,
    session::MapEditorSession,
    terrain_form::{self, TerrainDraft},
    theme_form::{self, ThemeDraft},
};

// The theme's display name is what the terrain and prefab writers build their folder from.
fn theme_display(session: &MapEditorSession, themes: Option<&UuidThemeRegistry>) -> String {
    themes
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone())
}

pub(in crate::net_qa::commands::write::save) fn save_terrain(
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    root: &Path,
) -> SaveOutcome {
    let uuid = draft.ensure_uuid();
    let display = theme_display(session, themes);
    SaveOutcome::from_result(terrain_form::write_terrain_in(root, draft, uuid, &display))
}

pub(in crate::net_qa::commands::write::save) fn save_theme(
    draft: &ThemeDraft,
    root: &Path,
) -> SaveOutcome {
    let key = draft.key();
    SaveOutcome::from_result(theme_form::write_theme_in(root, draft, key))
}

pub(in crate::net_qa::commands::write::save) fn save_prefab(
    map: &EditorMap,
    terrain: &TerrainDefRegistry,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    name: &EditorContentNameNet,
    root: &Path,
) -> SaveOutcome {
    let display = theme_display(session, themes);
    SaveOutcome::from_result(save::write_prefab_in(
        root, map, terrain, session, &display, name,
    ))
}
