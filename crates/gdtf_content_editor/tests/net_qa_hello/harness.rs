use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TestError};

pub(crate) fn editor_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

pub(crate) fn editor_state(app: &App) -> Option<EditorState> {
    app.world()
        .get_resource::<State<EditorState>>()
        .map(|state| state.get().clone())
}

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
