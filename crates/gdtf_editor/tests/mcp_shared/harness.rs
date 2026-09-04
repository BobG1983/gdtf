use bevy::prelude::*;
use cobalt_mcp_protocol::{
    message::{McpRequest, ProtocolVersion},
    ports::McpPort,
};
use gdtf_editor::{EditorState, MapEditorPlugin, McpEditorPlugin};
use gdtf_test_utils::{UiTestAppBuilder, advance_until};

use crate::{hello::assert_hello_ok, socket::Client, support::TestError};

/// An editor app on a real listener, built with no `EguiPlugin`. Without one the shell never
/// draws, so the form syncs it owns never run and each draft stays at its own default.
pub(crate) fn editor_app_listening() -> Result<(App, McpPort), TestError> {
    let (plugin, port) = McpEditorPlugin::listening(McpPort::new(0))?;
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
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
    advance_until(app, |app| editor_state(app) == Some(EditorState::Editing));
    for _ in 0..4 {
        app.update();
    }
}

/// An editing app with a negotiated client on its real listener.
pub(crate) fn editing_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, port) = editor_app_listening()?;
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &McpRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}
