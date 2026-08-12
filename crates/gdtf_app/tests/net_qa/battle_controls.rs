//! Selecting a fire mode on the shooter the mode panel would write it for.

use gdtf_app::qa_wire::misc::ModeKindNet;
use gdtf_battle_input::PathPreviewTarget;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    weapon::FireMode,
};
use gdtf_qa_protocol::{
    command::{CommandOutcome, RefusalNote, UnavailableCode},
    message::QaResponse,
};

use super::{
    battle_fixture::{
        arm_selected_with_modes, decoded, disarm_selected, down_the_players_gang,
        drive_into_battle_running, menu_app_with_net_qa, mount_on_selected, run_one_frame,
    },
    command_exchange::BATTLE_SET_FIRE_MODE,
    fire_mode_support::{
        BURST, FULL, FireModeBody, SINGLE, battle_with_a_burst_capable_gun, mode_argument,
        selected_mode, single_and_burst,
    },
};

/// A gun that offers full alone, which neither the shipped test weapon nor the carried one does.
fn full_only() -> FireMode {
    FireMode::new(vec![FULL])
}

/// The code and note a refused call carries.
fn refusal(answer: QaResponse) -> (UnavailableCode, RefusalNote) {
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = answer else {
        unreachable!("`{BATTLE_SET_FIRE_MODE}` must refuse this call, got {answer:?}");
    };
    (code, note)
}

#[test]
fn setting_burst_puts_the_gun_where_the_mode_panel_would() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_one_frame(
            &mut app,
            &tx,
            BATTLE_SET_FIRE_MODE,
            &mode_argument(ModeKindNet::Burst),
        ),
    );

    assert_eq!(
        body.mode,
        ModeKindNet::Burst,
        "the reply reports the mode the shooter is now on: {body:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(BURST),
        "the command sets the mode on the gun itself, so the gun is what says the mode panel and \
         a QA client end in the same place",
    );
}

#[test]
fn setting_the_mode_the_shooter_is_already_on_leaves_the_move_target_alone() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();
    let goal = CellLevel::new(Cell::new(14, 12), Level::new(0));
    app.world_mut()
        .insert_resource(PathPreviewTarget::new(goal));

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_one_frame(
            &mut app,
            &tx,
            BATTLE_SET_FIRE_MODE,
            &mode_argument(ModeKindNet::Single),
        ),
    );

    assert_eq!(
        body.mode,
        ModeKindNet::Single,
        "the fixture starts on the gun's single, so this call asks for the mode the shooter is \
         already on: {body:?}",
    );
    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        Some(goal),
        "a call that named the mode the shooter was already on changed no mode, so the move \
         target the player pinned has to still be pinned",
    );

    let _switched: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_one_frame(
            &mut app,
            &tx,
            BATTLE_SET_FIRE_MODE,
            &mode_argument(ModeKindNet::Burst),
        ),
    );

    assert_eq!(
        **app.world().resource::<PathPreviewTarget>(),
        None,
        "switching to a mode the shooter was NOT on clears the move target, so the case above is \
         reading a live reset and not a system that never runs here",
    );
}

#[test]
fn a_mode_the_gun_does_not_offer_is_refused_and_leaves_the_selection_alone() {
    let (mut app, tx) = battle_with_a_burst_capable_gun();

    let answer = run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Full),
    );

    let (code, note) = refusal(answer);
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "the gun is there and the shooter is selected; what is missing is the mode — {note:?}",
    );
    assert!(
        note.as_str().contains("does not offer that fire mode"),
        "the note has to name the mode as the missing piece, not the shooter or the gun, or a \
         client cannot tell which precondition to fix — {note:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(SINGLE),
        "a refused call writes nothing, so the shooter is still on the mode it was holding",
    );
}

#[test]
fn nobody_selected_is_refused_and_the_note_names_the_missing_shooter() {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    // The game re-picks a shooter every frame while one is standing, so the only way to reach
    // the handler with an empty selection is to leave it nobody living to pick.
    let _gang = down_the_players_gang(&mut app);

    let answer = run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Single),
    );

    let (code, note) = refusal(answer);
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "a battle is live and the resources are up; what is missing is the shooter — {note:?}",
    );
    assert!(
        note.as_str().contains("no shooter is selected"),
        "the note has to name the selection as the missing piece, not the gun or the mode, or a \
         client is sent to read a weapon spec off nobody — {note:?}",
    );
}

#[test]
fn a_shooter_holding_no_gun_is_refused_and_the_note_names_the_missing_weapon() {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    disarm_selected(&mut app);

    let answer = run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Single),
    );

    let (code, note) = refusal(answer);
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "a battle is live and a shooter is selected; what is missing is the gun — {note:?}",
    );
    assert!(
        note.as_str().contains("no weapon to fire"),
        "the note has to name the weapon as the missing piece, not the mode, or a client that \
         armed nothing is told to go and read a weapon spec that does not exist — {note:?}",
    );
}

#[test]
fn a_manned_mount_is_what_a_mode_is_set_on_not_the_gun_in_the_hands() {
    let (mut app, tx) = menu_app_with_net_qa();
    drive_into_battle_running(&mut app);
    let _carried = arm_selected_with_modes(&mut app, single_and_burst());
    let _mount = mount_on_selected(&mut app, full_only());
    app.update();

    let body: FireModeBody = decoded(
        BATTLE_SET_FIRE_MODE,
        run_one_frame(
            &mut app,
            &tx,
            BATTLE_SET_FIRE_MODE,
            &mode_argument(ModeKindNet::Full),
        ),
    );
    assert_eq!(
        body.mode,
        ModeKindNet::Full,
        "Full is offered by the MOUNT alone, and the mount is what the shooter fires: {body:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "the command sets the mode on the mount, so the mount is what says its mode was taken",
    );

    let (code, note) = refusal(run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Burst),
    ));
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "Burst is offered by the CARRIED gun alone, which is not the weapon that fires — {note:?}",
    );
    assert_eq!(
        selected_mode(&app),
        Some(FULL),
        "a refused call writes nothing, so the shooter is still on the mount's mode",
    );
}
