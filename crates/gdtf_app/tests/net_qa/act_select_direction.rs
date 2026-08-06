//! Which way the selection cycle walks, on a battlefield wide enough to tell one way from the other.
//!
//! With two gangers the ring is direction-blind — forward and back land on the same one — so this
//! case gives the harness three.

use std::sync::mpsc::Sender;

use bevy::app::App;
use gdtf_app::qa_wire::token::GangerToken;
use gdtf_battle_sim::test_support::fixtures;
use gdtf_net_qa_transport::IncomingRequest;

use super::{
    act_support::selected,
    battle_fixture::{drive_into_battle_running, menu_app_with_situation, run_request, send},
    battle_reads::player_gangers,
    command_exchange::{ACT_SELECT_NEXT, ACT_SELECT_PREV},
    socket_support::{TestError, TestResult},
};

/// Frames a selection command is given to reach the sim and answer.
const SETTLE_FRAMES: u32 = 16;

/// Step the cycle once and report who it landed on.
fn step_the_cycle(
    app: &mut App,
    tx: &Sender<IncomingRequest>,
    name: &'static str,
) -> Result<GangerToken, TestError> {
    let reply = send(tx, run_request(name, "()"));
    let mut answered = None;
    for _ in 0..SETTLE_FRAMES {
        app.update();
        if let Ok(answer) = reply.try_recv() {
            answered = Some(answer);
            break;
        }
    }
    let Some(answer) = answered else {
        return Err(format!("`{name}` never answered within {SETTLE_FRAMES} frames").into());
    };
    selected(name, answer)?
        .ok_or_else(|| format!("`{name}` must land on a ganger in a running battle").into())
}

#[test]
fn stepping_back_retraces_the_way_the_cycle_came() -> TestResult {
    let (mut app, tx) = menu_app_with_situation(fixtures::three_player_gangers());
    drive_into_battle_running(&mut app);

    let gangers = player_gangers(&app).len();
    assert!(
        gangers >= 3,
        "forward and back are the same step on a ring of two, so this case needs three player \
         gangers to mean anything; the fixture spawned {gangers}",
    );

    let first = step_the_cycle(&mut app, &tx, ACT_SELECT_NEXT)?;
    let second = step_the_cycle(&mut app, &tx, ACT_SELECT_NEXT)?;
    let third = step_the_cycle(&mut app, &tx, ACT_SELECT_NEXT)?;
    let back_to_second = step_the_cycle(&mut app, &tx, ACT_SELECT_PREV)?;
    let back_to_first = step_the_cycle(&mut app, &tx, ACT_SELECT_PREV)?;

    assert!(
        first != second && second != third && first != third,
        "three steps forward on a ring of three visit three different gangers: {first:?}, \
         {second:?}, {third:?}",
    );
    assert_eq!(
        back_to_second, second,
        "stepping back from the third lands on the second — stepping forward would have wrapped \
         to the first instead: {back_to_second:?}",
    );
    assert_eq!(
        back_to_first, first,
        "stepping back again lands on the first: {back_to_first:?}",
    );
    Ok(())
}
