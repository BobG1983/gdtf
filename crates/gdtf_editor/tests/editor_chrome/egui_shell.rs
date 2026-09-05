//! Editor shell: model resources exist in Editing; mode switches work.
use bevy::prelude::*;
use cobalt_test_utils::{UiTestAppBuilder, advance_until};
use gdtf_editor::{
    CanvasZoom, CurrentEditLevel, EditorMap, EditorMode, EditorState, HoveredCell, MapEditorPlugin,
    MapEditorSession,
};

fn editor_app() -> App {
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
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
fn editing_inserts_the_kept_model_resources() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let world = app.world();
    assert!(
        world.get_resource::<EditorMode>().is_some(),
        "the EditorMode resource (the active Workbench mode) must be inserted in Editing",
    );
    assert!(
        world.get_resource::<MapEditorSession>().is_some(),
        "the MapEditorSession (theme / size / selection) must be inserted in Editing",
    );
    assert!(
        world.get_resource::<EditorMap>().is_some(),
        "the EditorMap paintable model must be inserted in Editing",
    );
    assert!(
        world.get_resource::<CurrentEditLevel>().is_some(),
        "the CurrentEditLevel storey selector must be inserted in Editing",
    );
    assert!(
        world.get_resource::<CanvasZoom>().is_some(),
        "the CanvasZoom viewport-zoom factor must be inserted in Editing",
    );
    assert!(
        world.get_resource::<HoveredCell>().is_some(),
        "the HoveredCell model (written by the live hover + the QA capture) must be inserted in Editing",
    );
}

#[test]
fn editing_opens_in_the_default_prefab_mode() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    let mode = app.world().get_resource::<EditorMode>().copied();
    assert_eq!(
        mode,
        Some(EditorMode::Prefab),
        "the editor must open in the default PREFAB mode (the egui shell pre-selects that tab)",
    );
}

#[test]
fn mode_switch_mutates_the_editor_mode() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    set_mode(&mut app, EditorMode::Theme);
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Theme),
        "switching to THEME must mutate the EditorMode resource",
    );

    set_mode(&mut app, EditorMode::Terrain);
    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Terrain),
        "switching to TERRAIN must mutate the EditorMode resource",
    );
}

fn set_mode(app: &mut App, next: EditorMode) {
    let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() else {
        return;
    };
    *mode = next;
}
