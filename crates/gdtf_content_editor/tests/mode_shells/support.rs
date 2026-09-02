//! The editor app the mode-shell cases drive, and the wait that reaches Editing.
#![cfg(debug_assertions)]

use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A headless editor app with the real map-editor plugin on it.
pub(crate) fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
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
