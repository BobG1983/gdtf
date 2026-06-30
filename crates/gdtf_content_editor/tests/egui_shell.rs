//! Headless integration test for the GTW-512 egui editor shell.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`] rooted at the workspace `assets/`), so the editor's actual `Load` pass resolves
//! the shipped theme + content registries and its real `Editing` scene inserts the model resources —
//! not a copy.
//!
//! Per `verification.md` Rule 3 + the GTW-512 C1.7 contract, this asserts the STATE-MACHINE +
//! MODEL-RESOURCE contract — the editor reaches [`EditorState::Editing`]; the kept model resources
//! ([`EditorMode`], [`MapEditorSession`], [`EditorMap`], [`CurrentEditLevel`], [`CanvasZoom`],
//! [`HoveredCell`]) all exist; and a mode switch mutates [`EditorMode`]. It does NOT assert on
//! `bevy_ui` entities (they are GONE under egui), and it does NOT assert the egui DRAW — that is
//! verified by the gate's Screenshot-QA phase (the egui closure never runs headlessly without a
//! primary egui context, which is exactly why the draw is a screenshot concern, not a headless one).

use bevy::prelude::*;
use gdtf_content_editor::{
    CanvasZoom, CurrentEditLevel, EditorMap, EditorMode, EditorState, HoveredCell, MapEditorPlugin,
    MapEditorSession,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness: a live `AssetServer`
/// (rooted at the workspace `assets/`), a UI camera, and the editor's own [`MapEditorPlugin`]. This
/// is the SAME plugin the binary wires (minus the windowed [`EguiPlugin`](bevy_egui::EguiPlugin),
/// which needs a window the headless harness has none of — the egui draw is screenshot-verified, not
/// headless).
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drive the app until it reaches [`EditorState::Editing`] (its `Load` pass resolved the theme +
/// registries), then a few more frames so the `OnEnter(Editing)` inserts apply.
fn advance_to_editing(app: &mut App) {
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
         registries (a genuine load failure, not a frame-budget shortfall)",
    );
    // The `OnEnter(Editing)` inserts apply via command flush across a frame; advance a few more so
    // the model resources are present before we query.
    for _ in 0..4 {
        app.update();
    }
}

/// C1.7: the editor reaches `Editing` and every KEPT model resource the egui shell + its children
/// read is present after the `OnEnter(Editing)` insert chain (the state-scoped-resource pattern).
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
        "the HoveredCell model (GTW-512 C1.5 — written by the live hover + the QA capture) must be \
         inserted in Editing",
    );
}

/// C1.7: the editor opens in the default [`EditorMode::Prefab`] mode (the variant the egui shell
/// pre-selects), proving the inserted mode is the documented default.
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

/// C1.7: switching the mode MUTATES the [`EditorMode`] resource — the same write the egui mode tabs
/// and the `1`/`2`/`3` hotkeys perform. Driven directly on the resource (the egui tab click / the
/// hotkey both `set` the resource), so this pins the model contract the egui shell branches on.
#[test]
fn mode_switch_mutates_the_editor_mode() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    // Switch to THEME, then to TERRAIN — the resource must follow each write (the egui tabs + the
    // hotkeys both write this resource; the egui draw branches on it).
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

/// Set the [`EditorMode`] resource directly (the same write the egui mode tabs / the hotkeys make).
/// `let … else` keeps the test panic-free per the workspace lints (no `unwrap` / `expect`).
fn set_mode(app: &mut App, next: EditorMode) {
    let Some(mut mode) = app.world_mut().get_resource_mut::<EditorMode>() else {
        return;
    };
    *mode = next;
}
