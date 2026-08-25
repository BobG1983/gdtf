use bevy::app::App;
use gdtf_content_editor::{CurrentEditLevel, EditorMap};

use crate::support::TestError;

/// The painted map the world holds right now.
pub(crate) fn editor_map(app: &App) -> Result<EditorMap, TestError> {
    let Some(map) = app.world().get_resource::<EditorMap>() else {
        return Err("the painted map is a resource the editor creates on entering Editing".into());
    };
    Ok(map.clone())
}

/// The storey the canvas is painting on right now.
pub(crate) fn edit_level(app: &App) -> Result<CurrentEditLevel, TestError> {
    let Some(level) = app.world().get_resource::<CurrentEditLevel>() else {
        return Err("the edit storey is a resource the editor creates on entering Editing".into());
    };
    Ok(*level)
}
