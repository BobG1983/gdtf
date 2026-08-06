//! `act.throw_grenade` over a real socket: with no cell hovered the panel offers nothing.

use bevy::app::App;
use gdtf_app::qa_wire::act::ActRefusalNet;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::command::RunOptions;

use super::{
    super::{
        act_support::{caught_up, next},
        command_exchange::{ACT_THROW_GRENADE, exchange_inspecting, run},
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
    let _caught = next("wait", &mut replies)?;
    let reason = refused(ACT_THROW_GRENADE, next(ACT_THROW_GRENADE, &mut replies)?)?;
    assert_eq!(
        reason,
        ActRefusalNet::NoOffer,
        "the throw offer starts from the hovered cell, so a shooter with nothing hovered is \
         offered nothing and the command refuses rather than picking a cell of its own",
    );
    Ok(())
}
