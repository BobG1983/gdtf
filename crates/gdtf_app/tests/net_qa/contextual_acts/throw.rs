//! `act.throw_grenade` over a real socket: with no cell on offer the command refuses.

use bevy::app::App;
use gdtf_app::qa_wire::act::ActRefusalNet;
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};

use super::{
    super::{
        act_support::{assert_caught_up, caught_up, next},
        command_exchange::{ACT_THROW_GRENADE, WAIT, exchange_inspecting, run},
        socket_support::{TestError, TestResult, battle_app_listening},
    },
    scene::{refused, select_a_player_ganger, settle},
};

/// A live battle with a shooter selected and no cell hovered for a grenade to land on.
fn shooter_with_nothing_hovered() -> Result<(App, NetQaPort, ()), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (_shooter, _at) = select_a_player_ganger(&mut app)?;
    settle(&mut app);
    Ok((app, port, ()))
}

#[test]
fn throwing_a_grenade_refuses_with_no_offer_while_no_cell_is_hovered() -> TestResult {
    let (_app, replies, ()) = exchange_inspecting(shooter_with_nothing_hovered, |()| {
        vec![
            caught_up(),
            run(ACT_THROW_GRENADE, "()", RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let reason = refused(ACT_THROW_GRENADE, next(ACT_THROW_GRENADE, &mut replies)?)?;
    assert_eq!(
        reason,
        ActRefusalNet::NoOffer,
        "the command throws at the cell the panel is offering, so a shooter with no cell on \
         offer is refused rather than inventing a landing cell",
    );
    Ok(())
}
