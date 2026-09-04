//! Pixel hover over a real socket, on the windowless fixture the harness can build.

use cobalt_mcp_protocol::command::{RunOptions, UnavailableCode};

use super::{
    command_exchange::{INPUT_HOVER, exchange, run},
    input_support::refusal,
    socket_support::{TestResult, game_app_listening},
};

/// One pixel to hover, written in `input.hover`'s own argument shape.
const AT: &str = "(at:(x:120,y:48))";

#[test]
fn a_hover_on_a_host_with_no_primary_window_refuses_and_names_the_window() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(INPUT_HOVER, AT, RunOptions::default()),
    )?;
    let (code, note) = refusal(INPUT_HOVER, reply)?;

    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "this harness builds with `primary_window: None`, so a pixel hover has nowhere to land \
         — a wrong host state rather than a bad argument: {note}",
    );
    assert!(
        note.contains("Window"),
        "the refusal names the missing thing, so a caller can tell it from a bad pixel: {note}",
    );
    Ok(())
}
