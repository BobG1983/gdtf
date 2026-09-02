use bevy::app::App;
use gdtf_content_editor::EditorMode;

use crate::{
    drafts::theme_draft,
    harness::editing_app_and_client,
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
