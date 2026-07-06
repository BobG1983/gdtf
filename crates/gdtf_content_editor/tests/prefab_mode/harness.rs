//! The editor app + the advance-to-`Editing` driver.

use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll a SIGNAL, not a count.
pub(crate) const MAX_UPDATES: u32 = 10_000;

/// Build the real editor app on the no-renderer `DefaultPlugins` UI harness (the SAME plugin the
/// binary wires, minus the windowed `EguiPlugin` the headless harness has no window for — so the
/// egui image REGISTRATION no-ops, but the preview render-target + camera + tile sprites still
/// spawn, which is exactly the model contract this test asserts).
pub(crate) fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app
}

/// Drive the app to `Editing`, then a few frames so the `OnEnter(Editing)` inserts apply.
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
