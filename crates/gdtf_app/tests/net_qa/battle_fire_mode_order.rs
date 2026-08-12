//! A mode set over the wire has to outlast the selection sync that runs on the same frame.

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::qa_wire::misc::ModeKindNet;
use gdtf_battle_input::SelectedShooter;
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_qa_protocol::message::QaResponse;

use super::{
    battle_fixture::{decoded, run_request, selected_shooter, send},
    command_exchange::BATTLE_SET_FIRE_MODE,
    fire_mode_support::{
        FireModeBody, battle_with_a_burst_capable_gun, mode_argument, selected_mode,
    },
};

/// Send the command and drive the one frame on which the selection sync also has work to do.
///
/// Re-inserting the selection is what opens the sync's guard, so both writers land on this frame.
fn run_frame_against_the_sync(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    arguments: &str,
) -> QaResponse {
    let reply = send(tx, run_request(BATTLE_SET_FIRE_MODE, arguments));
    let shooter = selected_shooter(app);
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    app.update();
    let Ok(answer) = reply.try_recv() else {
        unreachable!(
            "`{BATTLE_SET_FIRE_MODE}` must answer inside the one frame that follows the send"
        );
    };
    answer
}

#[test]
fn a_set_mode_outlasts_the_selection_sync_on_the_same_frame() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_against_the_sync(&mut app, &tx, &mode_argument(ModeKindNet::Burst)),
    );

    assert_eq!(
        body.mode,
        ModeKindNet::Burst,
        "the reply names the mode the shooter is now on: {body:?}",
    );
    assert_eq!(
        selected_mode(&app),
        ModeKindNet::Burst,
        "the selection sync ran on this frame and copies the gun's single mode, so a Single here \
         means it landed after the command and the reply named a mode nobody is on",
    );
}

#[test]
fn a_set_mode_is_still_the_one_the_next_frame_prices() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();

    let _body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_against_the_sync(&mut app, &tx, &mode_argument(ModeKindNet::Burst)),
    );
    app.update();

    assert_eq!(
        selected_mode(&app),
        ModeKindNet::Burst,
        "every later price — cost, sightline, the fire act — reads this resource on a later \
         frame, so the mode has to still be Burst once the frame that set it is over",
    );
}
