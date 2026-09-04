//! The editor app the mode-shell cases drive, and the wait that reaches Editing.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

use bevy::prelude::*;
use gdtf_battle_sim::level::UuidThemeRegistry;
use gdtf_editor::{EditorState, MapEditorPlugin, MapEditorSession};
use gdtf_test_utils::{UiTestAppBuilder, advance_until};

/// A headless editor app with the real map-editor plugin on it.
pub(crate) fn editor_app() -> App {
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Advance until the editor's own state machine reaches Editing, then settle.
pub(crate) fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

/// The session theme's display name, which is the folder a terrain save writes into.
pub(crate) fn resolve_theme_display(app: &App) -> String {
    let Some(session) = app.world().get_resource::<MapEditorSession>() else {
        return String::new();
    };
    let theme = session.theme();
    app.world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|themes| themes.def(&theme).map(|def| (*def.display_name).clone()))
        .unwrap_or_default()
}
