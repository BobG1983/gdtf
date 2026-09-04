use bevy::prelude::*;
use gdtf_editor::{EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

pub(crate) fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

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
