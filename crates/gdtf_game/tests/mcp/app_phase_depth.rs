use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use cobalt_mcp_host::IncomingRequest;
use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
use gdtf_game::test_support::{AppState, BattleScapeState, GameState, RunningState};
use gdtf_test_utils::advance_until;

use super::{
    battle_fixture::{menu_app_with_mcp, request_battle, run_request, send},
    command_exchange::APP_PHASE,
};

fn read_phase(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> String {
    let reply = send(tx, run_request(APP_PHASE, "()"));
    app.update();
    app.update();
    let Ok(McpResponse::Outcome(CommandOutcome::Ran { reply, .. })) = reply.try_recv() else {
        unreachable!("a plain app.phase call must RUN");
    };
    reply.as_str().to_owned()
}

fn live_name<S: bevy::state::state::States + core::fmt::Debug>(app: &App) -> Option<String> {
    app.world()
        .get_resource::<State<S>>()
        .map(|state| format!("{:?}", state.get()))
}

#[test]
fn app_phase_reports_every_live_level_from_inside_a_battle() {
    let (mut app, tx) = menu_app_with_mcp();

    request_battle(&mut app);
    app.update();
    advance_until(&mut app, |app| {
        live_name::<BattleScapeState>(app).as_deref() == Some("BattleRunning")
    });

    let body = read_phase(&mut app, &tx);

    let Some(app_live) = live_name::<AppState>(&app) else {
        unreachable!("`app` is live inside a battle, so its State resource exists");
    };
    assert!(
        body.contains(&format!("app:{app_live}")),
        "`app` must report the app's own live state ({app_live}): {body}",
    );

    for (level, live) in [
        ("running", live_name::<RunningState>(&app)),
        ("game", live_name::<GameState>(&app)),
        ("battlescape", live_name::<BattleScapeState>(&app)),
    ] {
        let Some(live) = live else {
            unreachable!("`{level}` is live inside a battle, so its State resource exists");
        };
        assert!(
            body.contains(&format!("{level}:Some({live})")),
            "`{level}` must report the app's own live state ({live}): {body}",
        );
    }

    assert!(
        body.contains("battlescape:Some(BattleRunning)"),
        "the fixture rests the battle in BattleRunning: {body}",
    );
    assert!(
        body.contains("aftermath:None"),
        "the aftermath has not started, so its level is present and None: {body}",
    );
}
