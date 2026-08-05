use gdtf_app::test_support::NET_QA_SERVER_NAME;
use gdtf_qa_protocol::{
    command::{
        AwaitBudget, CommandAvailability, CommandName, CommandOutcome, CommandTiming, RunOptions,
    },
    message::{QaRequest, QaResponse},
};

use super::{
    command_exchange::{
        APP_PHASE, CAPTURE_SCREENSHOT, PLAYBACK_STATE, SETTINGS_READ, UI_FOCUS, exchange,
        exchange_all, run,
    },
    socket_support::{TestResult, game_app_listening},
};

#[test]
fn the_catalogue_lists_the_shell_read_set() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    assert_eq!(
        *catalogue.host, NET_QA_SERVER_NAME,
        "the catalogue must answer under the same host name the handshake reports: {catalogue:?}",
    );
    let published: Vec<CommandName> = catalogue
        .entries
        .iter()
        .map(|entry| entry.command.clone())
        .collect();
    assert_eq!(
        published,
        vec![
            CommandName::from_static(APP_PHASE),
            CommandName::from_static(CAPTURE_SCREENSHOT),
            CommandName::from_static(SETTINGS_READ),
            CommandName::from_static(UI_FOCUS),
            CommandName::from_static(PLAYBACK_STATE),
        ],
        "the game publishes exactly these commands today, in declaration order: {catalogue:?}",
    );
    for (name, timing) in [
        (APP_PHASE, CommandTiming::Immediate),
        (CAPTURE_SCREENSHOT, CommandTiming::Deferred),
        (SETTINGS_READ, CommandTiming::Immediate),
        (UI_FOCUS, CommandTiming::Immediate),
        (PLAYBACK_STATE, CommandTiming::Immediate),
    ] {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(entry.timing, timing, "{name} publishes the wrong timing");
        assert_eq!(entry.availability, CommandAvailability::Available);
        assert!(
            !entry.summary.as_str().is_empty(),
            "every row carries the one line a client reads to learn what it does",
        );
    }
    Ok(())
}

#[test]
fn running_app_phase_answers_ron_with_every_level_written() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(APP_PHASE, "()", RunOptions::default()),
    )?;
    let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain app.phase call must RUN, got {reply:?}");
    };
    let body = reply.as_str();

    assert!(
        ron::de::from_str::<ron::Value>(body).is_ok(),
        "the reply body is RON, not JSON: {body}",
    );
    for wanted in [
        "app:Running",
        "running:Some(Menu)",
        "game:None",
        "battlescape:None",
        "aftermath:None",
    ] {
        assert!(
            body.contains(wanted),
            "the reply writes every level explicitly, live or not — `{wanted}` is missing \
             from {body}",
        );
    }
    Ok(())
}

#[test]
fn a_misspelled_name_is_unknown_and_lists_what_exists() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run("app.phasee", "()", RunOptions::default()),
    )?;
    let QaResponse::Outcome(CommandOutcome::Unknown { known }) = reply else {
        unreachable!("a name this host does not offer must be Unknown, got {reply:?}");
    };
    assert_eq!(
        known,
        vec![
            CommandName::from_static(APP_PHASE),
            CommandName::from_static(CAPTURE_SCREENSHOT),
            CommandName::from_static(SETTINGS_READ),
            CommandName::from_static(UI_FOCUS),
            CommandName::from_static(PLAYBACK_STATE),
        ],
    );
    Ok(())
}

#[test]
fn an_unknown_argument_is_bad_arguments_with_the_schema() -> TestResult {
    let mut replies = exchange_all(
        game_app_listening,
        vec![
            QaRequest::Catalogue,
            run(APP_PHASE, "(nope:1)", RunOptions::default()),
        ],
    )?;
    let Some(outcome) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(QaResponse::Catalogue(catalogue)) = replies.pop() else {
        unreachable!("the first reply is the catalogue");
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(APP_PHASE))
    else {
        unreachable!("the catalogue carries the row the call names: {catalogue:?}");
    };

    let QaResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = outcome else {
        unreachable!("an argument the command does not declare must be refused, got {outcome:?}");
    };
    assert!(
        detail.as_str().contains("nope"),
        "the fault names the field that was wrong: {}",
        detail.as_str(),
    );
    assert_eq!(
        schema, entry.arguments,
        "the shape attached to a refusal is the SAME document the catalogue publishes",
    );
    Ok(())
}

#[test]
fn an_unbuilt_rider_is_refused_rather_than_dropped() -> TestResult {
    let mut replies = exchange_all(
        game_app_listening,
        vec![
            run(APP_PHASE, "()", RunOptions::default()),
            run(
                APP_PHASE,
                "()",
                RunOptions::new(Some(AwaitBudget::new(5)), None),
            ),
        ],
    )?;
    let Some(with_rider) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(plain) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    assert!(
        matches!(plain, QaResponse::Outcome(CommandOutcome::Ran { .. })),
        "the identical call WITHOUT the rider must run, or this case proves nothing about \
         the rider — got {plain:?}",
    );

    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = with_rider else {
        unreachable!("an await_ready rider must be refused, got {with_rider:?}");
    };
    assert_eq!(
        code,
        gdtf_qa_protocol::command::UnavailableCode::NotBuilt,
        "nothing about the app's state can make an unbuilt rider exist",
    );
    assert!(
        note.as_str().contains("await_ready"),
        "the refusal names the rider that is missing: {}",
        note.as_str(),
    );
    Ok(())
}
