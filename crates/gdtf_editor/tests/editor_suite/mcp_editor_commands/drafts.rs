use bevy::app::App;
use gdtf_editor::ThemeDraft;

use crate::mcp_shared::support::TestError;

/// The theme draft the world holds right now.
pub(crate) fn theme_draft(app: &App) -> Result<ThemeDraft, TestError> {
    let Some(draft) = app.world().get_resource::<ThemeDraft>() else {
        return Err("the theme draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}
