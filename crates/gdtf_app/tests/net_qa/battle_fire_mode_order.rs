//! A mode set over the wire has to outlast the other writers that run on the same frame.

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::{qa_wire::misc::ModeKindNet, test_support::ModeBurstButton};
use gdtf_battle_input::SelectedShooter;
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_qa_protocol::message::QaResponse;
use gdtf_test_utils::press_ui_button;

use super::{
    battle_fixture::{decoded, run_request, selected_shooter, send},
    command_exchange::BATTLE_SET_FIRE_MODE,
    fire_mode_support::{
        BURST, FULL, FireModeBody, SINGLE, battle_with_a_burst_capable_gun, battle_with_a_gun,
        mode_argument, mode_segment, selected_mode, single_burst_and_full,
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

/// Drive one frame that every writer of the mode has work on: sync, panel press, command.
///
/// The panel is pressed on Burst and the command asks for something else, so the mode the world
/// ends up holding names which writer ran last.
fn run_frame_with_every_writer(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    arguments: &str,
) -> QaResponse {
    let burst_segment = mode_segment::<ModeBurstButton>(app);
    press_ui_button(app, burst_segment);
    run_frame_against_the_sync(app, tx, arguments)
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
        Some(BURST),
        "the selection sync ran on this frame and copies the gun's single spec, so the gun's \
         single here means it landed after the command and the reply named a mode nobody is on",
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
        Some(BURST),
        "every later price — cost, sightline, the fire act — reads this resource on a later \
         frame, so the mode has to still be the gun's burst once the frame that set it is over",
    );
}

#[test]
fn a_set_mode_outlasts_a_mode_panel_press_on_the_same_frame() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_with_every_writer(&mut app, &tx, &mode_argument(ModeKindNet::Full)),
    );

    assert_eq!(
        body.mode,
        ModeKindNet::Full,
        "the reply names the mode the shooter is now on: {body:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "all three writers had work on this frame and the command ran last: the gun's burst here \
         means the panel press landed after it and the reply named a mode nobody is on, and the \
         gun's single means the selection sync did",
    );
}

#[test]
fn a_set_mode_beaten_by_no_writer_is_still_the_one_the_next_frame_prices() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());

    let _body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_with_every_writer(&mut app, &tx, &mode_argument(ModeKindNet::Full)),
    );
    app.update();

    assert_ne!(
        selected_mode(&app),
        Some(SINGLE),
        "the frame after a contested one must not have handed the mode back to the selection \
         sync's copy of the gun's single",
    );
    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "every later price reads this resource on a later frame, so the mode the command won has \
         to still be the one once the frame that set it is over",
    );
}
