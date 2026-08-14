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
        APP_PHASE, BATTLE_COST, BATTLE_FLEE, BATTLE_INSPECT, BATTLE_OFFERS, BATTLE_ROSTER,
        BATTLE_SELECTION, BATTLE_SIGHTLINE, BATTLE_START, BATTLE_TURN, BATTLE_VISIBLE,
        CAPTURE_SCREENSHOT, LOG_READ, PLAYBACK_STATE, PROCGEN_STEP, SETTINGS_READ, UI_FOCUS, WAIT,
        exchange, exchange_all, exchange_around, published_names, run,
    },
    socket_support::{
        TestResult, game_app_listening, let_the_descent_run, opening_battle_app_listening,
    },
};

/// Without the stepper compiled in, `procgen.step` is published but can never be built.
#[cfg(not(feature = "dev_tools"))]
pub(crate) const PROCGEN_STEP_IN_THE_MENU: UnavailableCode = UnavailableCode::NotBuilt;

/// With the stepper compiled in, the Menu is simply the wrong place to step generation.
#[cfg(feature = "dev_tools")]
pub(crate) const PROCGEN_STEP_IN_THE_MENU: UnavailableCode = UnavailableCode::WrongState;

#[test]
fn the_catalogue_lists_the_reads_the_lifecycle_commands_and_the_classic_and_contextual_acts()
-> TestResult {
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
        (BATTLE_COST, CommandTiming::Immediate),
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

/// Seconds the held call may keep re-testing; the frame budget runs out long before this does.
const OPENING_BUDGET: AwaitBudget = AwaitBudget::new(120);

#[test]
fn an_await_ready_rider_holds_the_call_until_the_battle_screen_opens() -> TestResult {
    let (before_it_opens, once_it_opens) = exchange_around(
        opening_battle_app_listening,
        run(BATTLE_TURN, "()", RunOptions::default()),
        let_the_descent_run,
        run(
            BATTLE_TURN,
            "()",
            RunOptions::new(Some(OPENING_BUDGET), None),
        ),
    )?;

    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = before_it_opens else {
        unreachable!(
            "the identical call WITHOUT the rider must refuse before the screen opens, or this \
             case proves nothing about the rider — got {before_it_opens:?}"
        );
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "off the battle screen the battle read refuses for its own reason, never for a missing \
         rider",
    );
    assert_eq!(
        note.as_str(),
        "this reads the battle screen, and the game is not on it",
    );

    assert!(
        matches!(
            once_it_opens,
            QaResponse::Outcome(CommandOutcome::Ran { .. })
        ),
        "the same call carrying await_ready is held and re-tested until the battle screen \
         opens, then runs — got {once_it_opens:?}",
    );
    Ok(())
}
