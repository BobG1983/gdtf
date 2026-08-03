use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

pub(crate) const MAX_UPDATES: u32 = 10_000;

pub(crate) fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

pub(crate) fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme + \
         registries",
    );
    for _ in 0..4 {
        app.update();
    }
}
