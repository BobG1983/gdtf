use bevy::app::App;
use gdtf_qa_protocol::{
    command::{CommandName, CommandOutcome},
    message::{ProtocolVersion, QaRequest, QaResponse},
};

use crate::{
    assertions::{assert_editor_catalogue, assert_hello_ok},
    client::{Client, EDITOR_PHASE, run_editor_phase},
    harness::{advance_to_editing, editor_app_listening, editor_state},
    load_case::reply_answered_during_load,
    phase_rows::{assert_modes_are_the_tab_order, assert_phase_names_the_live_state, decoded_ran},
    support::{TestError, TestResult},
};

/// An editing app with a negotiated client on its real listener.
fn editing_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, port) = editor_app_listening()?;
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}

#[test]
fn the_catalogue_names_the_one_command_the_editor_publishes() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &QaRequest::Catalogue)?;
    assert_editor_catalogue(&reply, editor_state(&app).as_ref());
    Ok(())
}

#[test]
fn a_run_while_editing_reports_the_live_phase_an_open_tab_and_every_tab() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor_phase("()"))?;
    let live = editor_state(&app);
    let body = decoded_ran(&reply)?;
    assert_phase_names_the_live_state(body.phase, live.as_ref());
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
    let (reply, during) =
        reply_answered_during_load(run_editor_phase("()"), "the editor.phase run")?;
    let body = decoded_ran(&reply)?;
    assert_phase_names_the_live_state(body.phase, during.as_ref());
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
