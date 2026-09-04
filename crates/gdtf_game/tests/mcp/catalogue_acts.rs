//! What the live catalogue says about the acts: timing, summary, and refusal.

use cobalt_mcp_protocol::{
    command::{CommandAvailability, CommandName, CommandTiming, UnavailableCode},
    message::{QaRequest, QaResponse},
};

use super::{
    command_exchange::{
        ACT_END_TURN, ACT_ENTER_EMPLACEMENT, ACT_EXECUTE, ACT_EXIT_EMPLACEMENT, ACT_FIRE,
        ACT_MELEE, ACT_MOVE, ACT_OPEN_DOOR, ACT_RELOAD, ACT_SELECT, ACT_SELECT_CLEAR,
        ACT_SELECT_NEXT, ACT_SELECT_PREV, ACT_SET_AIMING, ACT_SET_FACING, ACT_SET_STANCE,
        ACT_SHOVE, ACT_STABILIZE, ACT_THROW_GRENADE, exchange,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Every act, classic and contextual, with the timing its row must publish.
const ACT_TIMINGS: [(&str, CommandTiming); 19] = [
    (ACT_SELECT, CommandTiming::Immediate),
    (ACT_SELECT_NEXT, CommandTiming::Immediate),
    (ACT_SELECT_PREV, CommandTiming::Immediate),
    (ACT_SELECT_CLEAR, CommandTiming::Immediate),
    (ACT_MOVE, CommandTiming::Immediate),
    (ACT_FIRE, CommandTiming::Immediate),
    (ACT_RELOAD, CommandTiming::Immediate),
    (ACT_SET_STANCE, CommandTiming::Immediate),
    (ACT_SET_AIMING, CommandTiming::Immediate),
    (ACT_SET_FACING, CommandTiming::Immediate),
    (ACT_END_TURN, CommandTiming::Deferred),
    (ACT_MELEE, CommandTiming::Immediate),
    (ACT_SHOVE, CommandTiming::Immediate),
    (ACT_STABILIZE, CommandTiming::Immediate),
    (ACT_EXECUTE, CommandTiming::Immediate),
    (ACT_THROW_GRENADE, CommandTiming::Immediate),
    (ACT_OPEN_DOOR, CommandTiming::Immediate),
    (ACT_ENTER_EMPLACEMENT, CommandTiming::Immediate),
    (ACT_EXIT_EMPLACEMENT, CommandTiming::Immediate),
];

#[test]
fn the_catalogue_publishes_every_act_with_its_timing_and_a_summary() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for (name, timing) in ACT_TIMINGS {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(
            entry.timing, timing,
            "{name} publishes the wrong timing: only ending the turn holds its reply",
        );
        assert!(
            !entry.summary.as_str().is_empty(),
            "every row carries the one line a client reads to learn what it does: {entry:?}",
        );
    }
    Ok(())
}

#[test]
fn the_menu_catalogue_reads_every_act_as_refused() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for (name, _timing) in ACT_TIMINGS {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        let CommandAvailability::Unavailable { code, note } = &entry.availability else {
            unreachable!(
                "`{name}` cannot act from the Menu, so its live row must refuse: {entry:?}"
            )
        };
        assert_eq!(
            *code,
            UnavailableCode::WrongState,
            "no battle is running at the Menu, which is a wrong host state rather than a replay",
        );
        assert!(
            !note.as_str().is_empty(),
            "the refusal names the precondition that is missing",
        );
    }
    Ok(())
}
