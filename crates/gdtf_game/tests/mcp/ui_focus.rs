use bevy::{app::App, input_focus::InputFocus};
use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::QaResponse,
    ports::McpPort,
};
use gdtf_game::qa_wire::token::FocusTargetNet;
use serde::Deserialize;

use super::{
    command_exchange::{UI_FOCUS, exchange, run},
    socket_support::{TestError, TestResult, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct FocusBody {
    focused:   Option<FocusTargetNet>,
    focusable: Vec<FocusTargetNet>,
}

/// The menu app with keyboard focus dropped, leaving the screen's topology registered.
fn focus_cleared_app() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = game_app_listening()?;
    app.world_mut().resource_mut::<InputFocus>().clear();
    Ok((app, port))
}

fn focus_body(reply: QaResponse) -> FocusBody {
    let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain ui.focus call must RUN, got {reply:?}");
    };
    let body = reply.as_str();
    let Ok(focus) = ron::de::from_str::<FocusBody>(body) else {
        unreachable!("the reply body decodes into the published focus shape: {body}");
    };
    focus
}

#[test]
fn ui_focus_answers_the_menu_topology_and_the_widget_holding_focus() -> TestResult {
    let focus = focus_body(exchange(
        game_app_listening,
        run(UI_FOCUS, "()", RunOptions::default()),
    )?);

    assert_eq!(
        focus.focusable.len(),
        3,
        "the menu registers battlescape, options and quit as its directional topology — the \
         disabled hivescape button is deliberately never registered: {focus:?}",
    );
    let Some(focused) = focus.focused else {
        unreachable!("the menu sets initial focus as it spawns, so a widget holds it: {focus:?}");
    };
    assert!(
        focus.focusable.contains(&focused),
        "the focused widget must be one the screen registered as focusable: {focus:?}",
    );
    Ok(())
}

#[test]
fn ui_focus_lists_the_focusable_set_in_a_stable_order() -> TestResult {
    let focus = focus_body(exchange(
        game_app_listening,
        run(UI_FOCUS, "()", RunOptions::default()),
    )?);

    let ids: Vec<u64> = focus.focusable.iter().map(|target| **target).collect();
    assert!(
        ids.is_sorted(),
        "the nav map iterates in hash order, so the command sorts before answering — a client \
         polling ui.focus must see the same list in the same order every call: {ids:?}",
    );
    Ok(())
}

#[test]
fn ui_focus_reports_no_focused_widget_when_the_focus_resource_holds_none() -> TestResult {
    let focus = focus_body(exchange(
        focus_cleared_app,
        run(UI_FOCUS, "()", RunOptions::default()),
    )?);

    assert_eq!(
        focus.focusable.len(),
        3,
        "clearing focus leaves the menu's registered topology untouched: {focus:?}",
    );
    assert_eq!(
        focus.focused, None,
        "the focused field must come from `InputFocus`, not from the focusable set: this app \
         had its focus cleared before the command ran: {focus:?}",
    );
    Ok(())
}
