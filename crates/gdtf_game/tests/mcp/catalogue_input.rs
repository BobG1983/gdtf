//! What the live catalogue says about the raw-input commands: timing, summary, and availability.

use cobalt_mcp_protocol::{
    command::{CommandAvailability, CommandName, CommandTiming, UnavailableCode},
    message::{McpRequest, McpResponse},
};

use super::{
    command_exchange::{
        INPUT_ACTIVATE, INPUT_CLICK_CELL, INPUT_FOCUS_STEP, INPUT_HOVER, INPUT_PRESS_KEY,
        INPUT_SET_FOCUS, exchange,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Every raw-input command, with the timing its row must publish.
const INPUT_TIMINGS: [(&str, CommandTiming); 6] = [
    (INPUT_PRESS_KEY, CommandTiming::Deferred),
    (INPUT_HOVER, CommandTiming::Immediate),
    (INPUT_SET_FOCUS, CommandTiming::Immediate),
    (INPUT_FOCUS_STEP, CommandTiming::Immediate),
    (INPUT_ACTIVATE, CommandTiming::Immediate),
    (INPUT_CLICK_CELL, CommandTiming::Immediate),
];

/// The five commands that drive the shell, which answer on any screen.
const OFF_THE_BATTLE_SCREEN: [&str; 5] = [
    INPUT_PRESS_KEY,
    INPUT_HOVER,
    INPUT_SET_FOCUS,
    INPUT_FOCUS_STEP,
    INPUT_ACTIVATE,
];

#[test]
fn the_catalogue_publishes_every_raw_input_command_with_its_timing_and_a_summary() -> TestResult {
    let reply = exchange(game_app_listening, McpRequest::Catalogue)?;
    let McpResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for (name, timing) in INPUT_TIMINGS {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(
            entry.timing, timing,
            "{name} publishes the wrong timing: only the key press holds its reply, because it \
             answers once the release has been written",
        );
        assert!(
            !entry.summary.as_str().is_empty(),
            "every row carries the one line a client reads to learn what it does: {entry:?}",
        );
    }
    Ok(())
}

#[test]
fn the_menu_catalogue_offers_the_shell_input_and_refuses_the_click() -> TestResult {
    let reply = exchange(game_app_listening, McpRequest::Catalogue)?;
    let McpResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    for name in OFF_THE_BATTLE_SCREEN {
        let Some(entry) = catalogue
            .entries
            .iter()
            .find(|entry| entry.command == CommandName::from_static(name))
        else {
            unreachable!("the catalogue carries a row for {name}: {catalogue:?}");
        };
        assert_eq!(
            entry.availability,
            CommandAvailability::Available,
            "`{name}` drives the shell, which is exactly what a client needs at the Menu: \
             {entry:?}",
        );
    }

    let Some(click) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(INPUT_CLICK_CELL))
    else {
        unreachable!("the catalogue carries a row for {INPUT_CLICK_CELL}: {catalogue:?}");
    };
    let CommandAvailability::Unavailable { code, note } = &click.availability else {
        unreachable!("there is no battlescape to click at the Menu, so its row refuses: {click:?}")
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
    Ok(())
}
