//! The REAL editor app the suite queries, plus the drive-to-`Editing` driver (GTW-879).
//!
//! The same recipe `tests/prefab_mode/harness.rs` uses and 13 other editor test files already
//! follow: the no-renderer `DefaultPlugins` UI harness (a live
//! [`AssetServer`](bevy::asset::AssetServer) rooted at the workspace `assets/`) with the
//! editor's OWN [`MapEditorPlugin`] on top, so the editor's real `Load` asset pass resolves
//! the shipped registries and its real `OnEnter(Editing)` lifecycle inserts the authoring
//! model. Nothing about the model is hand-inserted here.
//!
//! On top of that this suite adds the QA channel plugin, pre-bound through
//! [`NetQaEditorPlugin::listening`] on port `0`. That constructor is still the right one: the
//! env gate the binary uses cannot be driven from a test (`std::env::set_var` is `unsafe` in
//! edition 2024 and the workspace forbids `unsafe`), and binding up front on an OS-assigned
//! port is what lets the client connect without racing the app's first `build` and keeps
//! parallel test binaries off each other's ports. Everything downstream of the bind — the
//! accept loop, the framing codec, the inbox, the drain — is the production path.

use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TestError};

/// Build the real editor app with its QA listener already bound, returning the app and the
/// port the OS assigned.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn editor_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    // The SAME plugin the binary wires, minus the windowed `EguiPlugin` the headless harness
    // has no window for — so the egui DRAW never runs, while the state machine, the `Load`
    // asset pass, the validation pass and the whole `Editing` model lifecycle do.
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

/// Drive the app until the editor's own `Load` pass releases into
/// [`EditorState::Editing`], then a few frames so the `OnEnter(Editing)` inserts apply.
///
/// While this runs the client half is polling `GetEditorQueryOptions`; each poll is answered
/// by one of these frames, which is exactly the wait-out-the-asset-pass loop the readiness
/// topic exists for.
pub(crate) fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|state| *state.get() == EditorState::Editing)
        },
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
