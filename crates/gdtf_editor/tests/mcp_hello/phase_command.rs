use cobalt_mcp_protocol::{
    command::{CommandName, CommandOutcome},
    message::{QaRequest, QaResponse},
};
use gdtf_editor::EditorState;

use crate::{
    assertions::{AnsweringPhase, assert_editor_catalogue},
    client::{EDITOR_PHASE, run_editor_phase},
    harness::{editing_app_and_client, editor_state},
    load_case::reply_answered_during_load,
    phase_rows::{assert_modes_are_the_tab_order, assert_phase_is, decoded_ran},
    support::TestResult,
};

#[test]
fn the_catalogue_names_every_command_the_editor_publishes() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let live = AnsweringPhase::of(editor_state(&app));
    let reply = client.exchange(&mut app, &QaRequest::Catalogue)?;
    assert_editor_catalogue(&reply, live);
    assert_eq!(
        live,
        AnsweringPhase::Editing,
        "the harness advanced this app to Editing before the request went out, and no path \
         returns to Load, so the phase the catalogue is read against is the world's own",
    );
    Ok(())
}

#[test]
fn a_run_while_editing_reports_the_live_phase_an_open_tab_and_every_tab() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor_phase("()"))?;
    let body = decoded_ran(&reply)?;
    let Some(live) = editor_state(&app) else {
        unreachable!("the editor's state machine exists for the whole of Editing");
    };
    assert_phase_is(body.phase, live);
    assert!(
        body.mode.is_some(),
        "the mode tab resource exists for the whole of Editing, so the reply names the open \
         tab rather than leaving it out: {body:?}",
    );
    assert_modes_are_the_tab_order(&body.modes);
    Ok(())
}

#[test]
fn a_run_during_load_reports_no_open_tab_and_still_every_tab() -> TestResult {
    let reply = reply_answered_during_load(run_editor_phase("()"), "the editor.phase run")?;
    let body = decoded_ran(&reply)?;
    assert_phase_is(body.phase, EditorState::Load);
    assert_eq!(
        body.mode, None,
        "the mode tab is a resource the editor only creates on entering Editing, so a Load-pass \
         reply reports no tab open rather than a default one: {body:?}",
    );
    assert_modes_are_the_tab_order(&body.modes);
    Ok(())
}

#[test]
fn an_unknown_argument_is_bad_arguments_carrying_the_published_schema() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
    let outcome = client.exchange(&mut app, &run_editor_phase("(nope:1)"))?;

    let QaResponse::Catalogue(catalogue) = catalogue else {
        unreachable!("a Catalogue request is answered with a catalogue, got {catalogue:?}");
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(EDITOR_PHASE))
    else {
        unreachable!("the catalogue carries a row for {EDITOR_PHASE}: {catalogue:?}");
    };
    let QaResponse::Outcome(CommandOutcome::BadArguments { schema, .. }) = &outcome else {
        unreachable!("a field the argument type does not declare is refused, got {outcome:?}");
    };
    assert_eq!(
        *schema, entry.arguments,
        "the refusal hands back the same argument shape the catalogue publishes, so one round \
         trip is enough to fix the call",
    );
    Ok(())
}
