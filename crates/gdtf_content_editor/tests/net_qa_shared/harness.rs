use bevy::prelude::*;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_qa_protocol::{
    message::{ProtocolVersion, QaRequest},
    ports::NetQaPort,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::{hello::assert_hello_ok, socket::Client, support::TestError};

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
    let hello = client.exchange(&mut app, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}
