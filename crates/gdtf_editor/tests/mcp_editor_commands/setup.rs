use std::path::Path;

use bevy::app::App;
use cobalt_mcp_protocol::{
    message::{McpRequest, ProtocolVersion},
    ports::McpPort,
};
use gdtf_editor::{EditorMode, MapEditorPlugin, McpEditorPlugin};
use gdtf_test_utils::GdtfUiTestAppBuilder;

use crate::{
    drafts::theme_draft,
    harness::{advance_to_editing, editing_app_and_client},
    hello::assert_hello_ok,
    mirror::ModeRow,
    names::{EDITOR_LOAD_THEME, EDITOR_SET_MODE},
    outcome::ran_body,
    rows::SetModeReplyRow,
    socket::{Client, run_editor},
    support::TestError,
    world::{editor_mode, session},
};

/// An editing app on the Theme tab with the session theme's def loaded into the draft.
///
/// The harness app runs no egui pass, so the form's own sync never fills the draft.
pub(crate) fn theme_tab_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    let theme = session(&app)?.theme();
    let opened = client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Theme)"))?;
    let opened: SetModeReplyRow = ran_body(&opened, EDITOR_SET_MODE)?;
    if opened.mode != ModeRow::Theme {
        return Err(format!(
            "the reply that opened the tab must name the Theme tab, got {:?}",
            opened.mode
        )
        .into());
    }
    let load = format!("(key: \"{}\")", *theme);
    client.exchange(&mut app, &run_editor(EDITOR_LOAD_THEME, &load))?;
    app.update();

    let mode = editor_mode(&app)?;
    if mode != EditorMode::Theme {
        return Err(
            format!("the Theme tab must be open before a draft case runs, got {mode:?}").into(),
        );
    }
    let draft = theme_draft(&app)?;
    if draft.key() != theme {
        return Err(format!(
            "the draft must hold the session theme's own def before a draft case runs: {:?} \
             against {:?}",
            draft.key(),
            theme,
        )
        .into());
    }
    Ok((app, client))
}

/// An editor app on a real listener whose asset server reads `root`.
///
/// The shared harness reads the workspace assets, so a delete driven against a temp
/// QA assets root would remove nothing and assert nothing.
pub(crate) fn editor_app_listening_on(root: &Path) -> Result<(App, McpPort), TestError> {
    let (plugin, port) = McpEditorPlugin::listening(McpPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::on_asset_root(root)
        .with_ui_camera()
        .build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

/// An editing app on `root` with a negotiated client on its real listener.
pub(crate) fn editing_app_and_client_on(root: &Path) -> Result<(App, Client), TestError> {
    let (mut app, port) = editor_app_listening_on(root)?;
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &McpRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}
