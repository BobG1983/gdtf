//! What the live catalogue says about the view and battle controls: timing and availability.

use gdtf_qa_protocol::{
    command::{CommandAvailability, CommandName, CommandTiming, UnavailableCode},
    message::{QaRequest, QaResponse},
};

use super::{
    command_exchange::{
        BATTLE_SET_FIRE_MODE, VIEW_LEVEL_DOWN, VIEW_LEVEL_UP, VIEW_LOOK_AT, VIEW_PAN,
        VIEW_TOGGLE_FULL_VIEW, exchange,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Every view and battle control, with the timing its row must publish.
const CONTROL_TIMINGS: [(&str, CommandTiming); 6] = [
    (VIEW_LEVEL_UP, CommandTiming::Immediate),
    (VIEW_LEVEL_DOWN, CommandTiming::Immediate),
    (VIEW_TOGGLE_FULL_VIEW, CommandTiming::Immediate),
    (VIEW_PAN, CommandTiming::Deferred),
    (VIEW_LOOK_AT, CommandTiming::Deferred),
    (BATTLE_SET_FIRE_MODE, CommandTiming::Immediate),
];

#[test]
fn the_catalogue_publishes_every_control_with_its_timing_and_a_summary() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for (name, timing) in CONTROL_TIMINGS {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(
            entry.timing, timing,
            "{name} publishes the wrong timing: only the two camera moves hold their reply, \
             because they answer once the frame's bounds clamp has run",
        );
        assert!(
            !entry.summary.as_str().is_empty(),
            "every row carries the one line a client reads to learn what it does: {entry:?}",
        );
    }
    Ok(())
}

#[test]
fn every_control_refuses_at_the_menu() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for (name, _timing) in CONTROL_TIMINGS {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        let CommandAvailability::Unavailable { code, note } = &entry.availability else {
            unreachable!("there is no battle at the Menu, so `{name}` refuses: {entry:?}");
        };
        assert_eq!(
            *code,
            UnavailableCode::WrongState,
            "no battle is live at the Menu, which is a wrong host state rather than a replay: \
             `{name}` — {note:?}",
        );
        assert!(
            !note.as_str().is_empty(),
            "the refusal names the precondition that is missing: `{name}`",
        );
    }
    Ok(())
}
