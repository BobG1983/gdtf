//! The REAL editor app the suite handshakes with, plus the drive-to-`Editing` driver
//! (GTW-879; split out of the flat file in GTW-896).

use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TestError};

/// Build the real editor app with its QA listener already bound, returning the app and the
/// port the OS assigned.
///
/// The listener is pre-bound through [`NetQaEditorPlugin::listening`] because the env gate the
/// binary uses cannot be driven from a test (`std::env::set_var` is `unsafe` in edition 2024
/// and the workspace forbids `unsafe`); everything downstream of the bind is the production
/// path. [`MapEditorPlugin`] is the SAME plugin the binary wires, minus the windowed
/// [`EguiPlugin`](bevy_egui::EguiPlugin) the headless harness has no window for.
///
/// The app returned has run NO frames, so it sits in [`EditorState::Load`] — the state the
/// GTW-896 case observes.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn editor_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

/// The editor's current lifecycle state, read off the world the app itself maintains.
///
/// `None` only if the state machine is missing entirely, which would mean
/// [`MapEditorPlugin`] never ran — the callers assert on that rather than assuming it.
pub(crate) fn editor_state(app: &App) -> Option<EditorState> {
    app.world()
        .get_resource::<State<EditorState>>()
        .map(|state| state.get().clone())
}

/// Drive the app until the editor's own `Load` pass releases into [`EditorState::Editing`],
/// then a few frames so the `OnEnter(Editing)` inserts apply.
pub(crate) fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| editor_state(app) == Some(EditorState::Editing),
        EDITING_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..4 {
        app.update();
    }
}
