use gdtf_content_editor::EditorMode;

use crate::{
    client::{EDITOR_SET_MODE, run_editor, run_editor_phase},
    harness::editing_app_and_client,
    lifecycle::unavailable_code,
    load_case::reply_answered_during_load,
    phase_rows::{ModeRow, decoded_ran},
    rows::{SetModeReplyRow, ran_body},
    support::{TestError, TestResult},
};

const WANTED: EditorMode = EditorMode::Armor;

fn live_mode(app: &bevy::app::App) -> Result<EditorMode, TestError> {
    let Some(mode) = app.world().get_resource::<EditorMode>() else {
        return Err("the mode tab is a resource the editor creates on entering Editing".into());
    };
    Ok(*mode)
}

#[test]
fn set_mode_opens_the_tab_the_world_and_the_phase_read_then_report() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    assert_ne!(
        live_mode(&app)?,
        WANTED,
        "the case must ask for a tab the editor is not already on, or it would pass against a \
         handler that writes nothing",
    );

    let reply = client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Armor)"))?;
    let body: SetModeReplyRow = ran_body(&reply, EDITOR_SET_MODE)?;

    assert_eq!(
        body.mode,
        ModeRow::Armor,
        "the reply names the tab that was opened",
    );
    assert_eq!(
        live_mode(&app)?,
        WANTED,
        "the world's own mode tab is the requested one on the frame that answered — this is the \
         same resource the tab bar and the number hotkeys write",
    );

    let phase = client.exchange(&mut app, &run_editor_phase("()"))?;
    let phase_body = decoded_ran(&phase)?;
    assert_eq!(
        phase_body.mode,
        Some(ModeRow::Armor),
        "a following editor.phase reports the tab set_mode opened as the active one",
    );
    Ok(())
}

#[test]
fn set_mode_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply = reply_answered_during_load(
        run_editor(EDITOR_SET_MODE, "(mode: Armor)"),
        "the editor.set_mode run",
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the mode tab is state-scoped to Editing, so a Load-pass call is refused rather than \
         creating one",
    );
    Ok(())
}
