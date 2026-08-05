use gdtf_app::test_support::NET_QA_SERVER_NAME;
use gdtf_qa_protocol::{
    command::{
        AwaitBudget, CommandAvailability, CommandName, CommandOutcome, CommandTiming, RunOptions,
        UnavailableCode,
    },
    message::{QaRequest, QaResponse},
};

use super::{
    command_exchange::{
        APP_PHASE, BATTLE_FLEE, BATTLE_INSPECT, BATTLE_OFFERS, BATTLE_ROSTER, BATTLE_SELECTION,
        BATTLE_SIGHTLINE, BATTLE_START, BATTLE_TURN, BATTLE_VISIBLE, CAPTURE_SCREENSHOT, LOG_READ,
        PLAYBACK_STATE, PROCGEN_STEP, SETTINGS_READ, UI_FOCUS, WAIT, exchange, exchange_all, run,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Without the stepper compiled in, `procgen.step` is published but can never be built.
#[cfg(not(feature = "dev_tools"))]
pub(crate) const PROCGEN_STEP_IN_THE_MENU: UnavailableCode = UnavailableCode::NotBuilt;

/// With the stepper compiled in, the Menu is simply the wrong place to step generation.
#[cfg(feature = "dev_tools")]
pub(crate) const PROCGEN_STEP_IN_THE_MENU: UnavailableCode = UnavailableCode::WrongState;

/// Every command the game publishes, in declaration order.
pub(crate) fn published_names() -> Vec<CommandName> {
    vec![
        CommandName::from_static(APP_PHASE),
        CommandName::from_static(CAPTURE_SCREENSHOT),
        CommandName::from_static(SETTINGS_READ),
        CommandName::from_static(UI_FOCUS),
        CommandName::from_static(PLAYBACK_STATE),
        CommandName::from_static(BATTLE_ROSTER),
        CommandName::from_static(BATTLE_TURN),
        CommandName::from_static(BATTLE_SELECTION),
        CommandName::from_static(BATTLE_OFFERS),
        CommandName::from_static(BATTLE_INSPECT),
        CommandName::from_static(BATTLE_SIGHTLINE),
        CommandName::from_static(BATTLE_VISIBLE),
        CommandName::from_static(LOG_READ),
        CommandName::from_static(BATTLE_START),
        CommandName::from_static(BATTLE_FLEE),
        CommandName::from_static(PROCGEN_STEP),
        CommandName::from_static(WAIT),
    ]
}

#[test]
fn the_catalogue_lists_the_shell_reads_the_battle_reads_and_the_lifecycle_commands() -> TestResult {
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
        published_names(),
        "the game publishes exactly these commands today, in declaration order: {catalogue:?}",
    );
    for (name, timing) in [
        (APP_PHASE, CommandTiming::Immediate),
        (CAPTURE_SCREENSHOT, CommandTiming::Deferred),
        (SETTINGS_READ, CommandTiming::Immediate),
        (UI_FOCUS, CommandTiming::Immediate),
        (PLAYBACK_STATE, CommandTiming::Immediate),
        (BATTLE_START, CommandTiming::Deferred),
        (BATTLE_FLEE, CommandTiming::Deferred),
        (PROCGEN_STEP, CommandTiming::Immediate),
        (WAIT, CommandTiming::Deferred),
    ] {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(entry.timing, timing, "{name} publishes the wrong timing");
        assert!(
            !entry.summary.as_str().is_empty(),
            "every row carries the one line a client reads to learn what it does",
        );
    }
    Ok(())
}

#[test]
fn the_catalogue_refuses_every_battle_read_at_the_menu() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for name in [
        BATTLE_ROSTER,
        BATTLE_TURN,
        BATTLE_SELECTION,
        BATTLE_OFFERS,
        BATTLE_INSPECT,
        BATTLE_SIGHTLINE,
        BATTLE_VISIBLE,
        LOG_READ,
    ] {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert!(
            matches!(
                entry.availability,
                CommandAvailability::Unavailable {
                    code: gdtf_qa_protocol::command::UnavailableCode::WrongState,
                    ..
                }
            ),
            "the catalogue's availability column is the live answer, so `{name}` reads as \
             refused at the menu: {entry:?}",
        );
        assert!(
            entry.timing == CommandTiming::Immediate,
            "every battle read answers inside the frame it is claimed in: {entry:?}",
        );
    }
    Ok(())
}

#[test]
fn the_menu_catalogue_scopes_availability_to_what_the_menu_can_actually_do() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    let availability_of = |name: &'static str| {
        catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
            .map(|entry| entry.availability.clone())
    };

    for name in [
        APP_PHASE,
        CAPTURE_SCREENSHOT,
        SETTINGS_READ,
        UI_FOCUS,
        PLAYBACK_STATE,
        BATTLE_START,
        WAIT,
    ] {
        assert_eq!(
            availability_of(name),
            Some(CommandAvailability::Available),
            "{name} answers from the Menu, so the live catalogue must say so: {catalogue:?}",
        );
    }

    for (name, code) in [
        (BATTLE_FLEE, UnavailableCode::WrongState),
        (PROCGEN_STEP, PROCGEN_STEP_IN_THE_MENU),
    ] {
        let Some(CommandAvailability::Unavailable { code: live, note }) = availability_of(name)
        else {
            unreachable!(
                "{name} cannot run in the Menu, so its live row must be Unavailable: {catalogue:?}"
            );
        };
        assert_eq!(live, code, "{name} must name why it cannot run");
        assert!(
            !note.as_str().is_empty(),
            "{name}'s refusal must name the precondition that is missing",
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
    assert_eq!(known, published_names());
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
        UnavailableCode::NotBuilt,
        "nothing about the app's state can make an unbuilt rider exist",
    );
    assert!(
        note.as_str().contains("await_ready"),
        "the refusal names the rider that is missing: {}",
        note.as_str(),
    );
    Ok(())
}
