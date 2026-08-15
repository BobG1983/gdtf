//! Editing-scoped resources seed on enter `Editing` and remove on exit.
mod asserts;

use bevy::prelude::*;
use gdtf_content_editor::{EditorMode, EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::asserts::{assert_all_scoped_resources_absent, assert_all_scoped_resources_seeded};

fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(bevy::render::sync_world::SyncWorldPlugin);
    app.add_plugins(MapEditorPlugin);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn editing_scoped_resources_seed_on_enter_and_remove_on_exit() {
    let mut app = editor_app();

    app.update();
    assert_eq!(
        app.world()
            .get_resource::<State<EditorState>>()
            .map(|s| s.get().clone()),
        Some(EditorState::Load),
        "the editor boots into its Load pass",
    );
    assert_all_scoped_resources_absent(&app, "must be absent while the editor is still loading");

    advance_to_editing(&mut app);
    assert_all_scoped_resources_seeded(&app);

    app.world_mut()
        .resource_mut::<NextState<EditorState>>()
        .set(EditorState::Load);
    app.update();
    assert_all_scoped_resources_absent(&app, "must be removed once Editing exits");

    advance_to_editing(&mut app);
    assert_eq!(
        app.world().get_resource::<EditorMode>(),
        Some(&EditorMode::default()),
        "a re-entered Editing span must re-seed its scoped resources",
    );
}
