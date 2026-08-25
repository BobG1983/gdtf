use bevy::app::App;
use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainDefRegistry};
use gdtf_content_editor::{EditorMode, MapEditorSession, ThemeDraft};

use crate::support::TestError;

/// The authoring session the world holds right now.
pub(crate) fn session(app: &App) -> Result<MapEditorSession, TestError> {
    let Some(session) = app.world().get_resource::<MapEditorSession>() else {
        return Err("the session is a resource the editor creates on entering Editing".into());
    };
    Ok(session.clone())
}

/// The theme draft the world holds right now.
pub(crate) fn theme_draft(app: &App) -> Result<ThemeDraft, TestError> {
    let Some(draft) = app.world().get_resource::<ThemeDraft>() else {
        return Err("the theme draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The mode tab the world has open right now.
pub(crate) fn editor_mode(app: &App) -> Result<EditorMode, TestError> {
    let Some(mode) = app.world().get_resource::<EditorMode>() else {
        return Err("the mode tab is a resource the editor creates on entering Editing".into());
    };
    Ok(*mode)
}

/// The terrain registry the editor loaded before it entered Editing.
pub(crate) fn terrain_registry(app: &App) -> Result<TerrainDefRegistry, TestError> {
    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        return Err("the editor reached Editing, so its terrain registry is loaded".into());
    };
    Ok(registry.clone())
}

/// The theme registry the editor loaded before it entered Editing.
pub(crate) fn theme_registry(app: &App) -> Result<UuidThemeRegistry, TestError> {
    let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() else {
        return Err("the editor reached Editing, so its theme registry is loaded".into());
    };
    Ok(registry.clone())
}
