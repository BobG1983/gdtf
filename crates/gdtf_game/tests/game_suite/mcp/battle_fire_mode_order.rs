//! A mode set over the wire has to outlast the other writer that runs on the same frame.

use std::sync::mpsc;

use bevy::app::App;
use cobalt_mcp_host::IncomingRequest;
use cobalt_mcp_protocol::message::McpResponse;
use cobalt_test_utils::press_ui_button;
use gdtf_game::{qa_wire::misc::ModeKindNet, test_support::ModeBurstButton};

use super::{
    battle_fixture::{decoded, run_request, send},
    command_exchange::BATTLE_SET_FIRE_MODE,
    fire_mode_support::{
        BURST, FULL, FireModeBody, battle_with_a_burst_capable_gun, battle_with_a_gun,
        mode_argument, mode_segment, selected_mode, single_burst_and_full,
    },
};

/// Send the command and drive the one frame that answers it.
fn run_frame(app: &mut App, tx: &mpsc::Sender<IncomingRequest>, arguments: &str) -> McpResponse {
    let reply = send(tx, run_request(BATTLE_SET_FIRE_MODE, arguments));
    app.update();
    let Ok(answer) = reply.try_recv() else {
        unreachable!(
            "`{BATTLE_SET_FIRE_MODE}` must answer inside the one frame that follows the send"
        );
    };
    answer
}

/// Drive one frame both writers of the mode have work on: a panel press, then the command.
///
/// The panel is pressed on Burst and the command asks for something else, so the mode the gun
/// ends up on names which writer ran last.
fn run_frame_with_both_writers(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    arguments: &str,
) -> McpResponse {
    let burst_segment = mode_segment::<ModeBurstButton>(app);
    press_ui_button(app, burst_segment);
    run_frame(app, tx, arguments)
}

#[test]
fn a_set_mode_is_still_the_one_the_next_frame_prices() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();

    let _body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame(&mut app, &tx, &mode_argument(ModeKindNet::Burst)),
    );
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(BURST),
        "every later price — cost, sightline, the fire act — reads the gun on a later frame, so \
         the mode has to still be the gun's burst once the frame that set it is over",
    );
}

#[test]
fn a_set_mode_outlasts_a_mode_panel_press_on_the_same_frame() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_with_both_writers(&mut app, &tx, &mode_argument(ModeKindNet::Full)),
    );

    assert_eq!(
        body.mode,
        ModeKindNet::Full,
        "the reply names the mode the shooter is now on: {body:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "both writers had work on this frame and the command ran last: the gun's burst here \
         means the panel press landed after it and the reply named a mode nobody is on",
    );
}

#[test]
fn a_set_mode_beaten_by_no_writer_is_still_the_one_the_next_frame_prices() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());

    let _body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_frame_with_both_writers(&mut app, &tx, &mode_argument(ModeKindNet::Full)),
    );
    app.update();

    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "every later price reads the gun on a later frame, so the mode the command won has to \
         still be the one once the frame that set it is over",
    );
}
