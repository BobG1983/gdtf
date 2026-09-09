//! An act the QA layer can turn down itself is refused, rather than reaching the sim.

use std::sync::mpsc::Receiver;

use bevy::app::App;
use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
use gdtf_battle_input::SelectedShooter;

use crate::mcp::{
    battle_fixture::{
        down_the_players_gang, drive_into_battle_running, menu_app_with_mcp, run_request, send,
    },
    battle_reads::ganger_argument,
    command_exchange::{ACT_MOVE, ACT_RELOAD, ACT_SELECT, ACT_SET_AIMING, ACT_SET_STANCE},
    wait::support::answered_within,
};

/// Any cell will do: the act never reaches the sim, so the destination is never read.
const SOMEWHERE: &str = "(at:(cell:(x:1,y:1),level:0))";

/// Step until the reply lands, then hand back its RON body.
fn body_of(app: &mut App, reply: &Receiver<McpResponse>, named: &str) -> String {
    let answer = answered_within(app, reply);
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = answer else {
        unreachable!("`{named}` must answer rather than refuse admission: {answer:?}");
    };
    reply.as_str().to_owned()
}

/// Drive a battle whose gang is all down, then run one act against it.
fn refuse_with_nobody_selected(named: &'static str, arguments: &str) {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let _gang = down_the_players_gang(&mut app);

    let reply = send(&tx, run_request(named, arguments));
    let body = body_of(&mut app, &reply, named);
    assert!(
        body.contains("Refused") && body.contains("NoShooter"),
        "`{named}` needs a selected shooter and there is none, so it must be refused NoShooter \
         without touching the sim; got {body}",
    );
    assert!(
        app.world().resource::<SelectedShooter>().is_none(),
        "`{named}` must not select anyone on its way to refusing",
    );
}

#[test]
fn selecting_a_ganger_that_is_down_is_refused_unknown_token() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let gang = down_the_players_gang(&mut app);
    let Some(downed) = gang.first().copied() else {
        unreachable!("downing the gang reports who it took away");
    };

    let reply = send(&tx, run_request(ACT_SELECT, &ganger_argument(downed)));
    let body = body_of(&mut app, &reply, ACT_SELECT);
    assert!(
        body.contains("Refused") && body.contains("UnknownToken"),
        "the game never selects a ganger that is down, so its token must come back refused \
         UnknownToken rather than being accepted and quietly dropped; got {body}",
    );
    assert!(
        app.world().resource::<SelectedShooter>().is_none(),
        "a refused token must not select anyone",
    );
}

#[test]
fn reloading_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_RELOAD, "()");
}

#[test]
fn a_posture_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_SET_STANCE, "(stance:Prone)");
    refuse_with_nobody_selected(ACT_SET_AIMING, "(aim:true)");
}

#[test]
fn a_move_with_nobody_selected_is_refused_no_shooter() {
    refuse_with_nobody_selected(ACT_MOVE, SOMEWHERE);
}
