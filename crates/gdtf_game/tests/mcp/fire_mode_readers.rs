//! Everything that reads the selected fire mode has to see the write that landed on its frame.

use std::sync::mpsc;

use bevy::app::App;
use cobalt_mcp_protocol::message::{QaRequest, QaResponse};
use cobalt_mcp_transport::IncomingRequest;
use gdtf_battle_input::{InspectTarget, SelectedShooter};
use gdtf_battle_presenter::FireTargetHighlight;
use gdtf_battle_sim::{
    acts::fire_arc_tu_cost,
    ganger::{Aiming, Facing, LifeState, Position, Tu, TuMax},
    magazine::mode_tu_cost,
    prelude::CellLevel,
    tuning::CombatTuning,
    weapon::FireModeSpec,
};
use gdtf_game::{
    qa_wire::{act::ActReply, misc::ModeKindNet, refusal::ShotRefusalNet},
    test_support::{ModeBurstButton, ModeSingleButton},
};

use super::{
    battle_fixture::{decoded, run_one_frame, run_request, selected_shooter, send},
    battle_reads::{an_enemy_ganger_at, cell_argument, cell_of},
    battle_selection::SelectionBody,
    command_exchange::{ACT_FIRE, BATTLE_SELECTION, BATTLE_SET_FIRE_MODE},
    fire_mode_support::{
        FULL, SINGLE, battle_with_a_gun, mode_argument, mode_segment_is_active,
        single_burst_and_full,
    },
    magazine_support::fill_the_magazine,
};

/// Ask for a mode and read something back on the very same frame.
fn ask_while_setting(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    mode: ModeKindNet,
    read: QaRequest,
) -> QaResponse {
    let _set = send(tx, run_request(BATTLE_SET_FIRE_MODE, &mode_argument(mode)));
    let reply = send(tx, read);
    app.update();
    let Ok(answer) = reply.try_recv() else {
        unreachable!("the read must answer inside the one frame that follows the send");
    };
    answer
}

/// What the sim charges the selected shooter for one shot at `at` in `spec`.
fn shot_price(app: &App, spec: &FireModeSpec, at: CellLevel) -> Tu {
    let world = app.world();
    let shooter = selected_shooter(app);
    let (Ok(row), Some(tuning)) = (
        world.get_entity(shooter),
        world.get_resource::<CombatTuning>(),
    ) else {
        unreachable!("a running battle holds its tuning and the shooter it selected");
    };
    let (Some(tu_max), Some(aiming), Some(facing), Some(position)) = (
        row.get::<TuMax>(),
        row.get::<Aiming>(),
        row.get::<Facing>(),
        row.get::<Position>(),
    ) else {
        unreachable!("a selected shooter carries the pose a shot is priced against");
    };
    fire_arc_tu_cost(
        **facing,
        position.cell(),
        at.cell(),
        mode_tu_cost(spec, tu_max, aiming, tuning),
        tuning,
    )
}

#[test]
fn the_selection_read_answers_with_the_mode_set_on_its_own_frame() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());

    let body: SelectionBody = decoded(
        BATTLE_SELECTION,
        ask_while_setting(
            &mut app,
            &tx,
            ModeKindNet::Full,
            run_request(BATTLE_SELECTION, "()"),
        ),
    );

    assert_eq!(
        body.fire_mode,
        Some(ModeKindNet::Full),
        "battle.selection is what a client reads the live mode with, so answering the mode a \
         write on the same frame replaced hands back a mode nobody is on: {body:?}",
    );
}

#[test]
fn the_mode_panel_marks_the_mode_set_on_its_own_frame() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());
    assert!(
        mode_segment_is_active::<ModeSingleButton>(&mut app),
        "the case starts on the gun's single, so a burst mark later is the write it is asked \
         about and not the mark it began with",
    );

    let _answer = run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Burst),
    );

    assert!(
        mode_segment_is_active::<ModeBurstButton>(&mut app),
        "the panel marks the mode the shooter is on, so a single still marked at the end of the \
         frame that set burst shows the player a segment nobody is on",
    );
}

#[test]
fn a_shot_asked_for_on_the_same_frame_is_priced_with_the_mode_just_set() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());
    let shooter = selected_shooter(&app);
    let _rounds = fill_the_magazine(&mut app, shooter);
    let Some(at) = cell_of(&app, shooter) else {
        unreachable!("the selected shooter stands somewhere on the grid");
    };
    let affordable = shot_price(&app, &SINGLE, at.to_sim());
    let Ok(mut row) = app.world_mut().get_entity_mut(shooter) else {
        unreachable!("the selected shooter is a live entity");
    };
    row.insert((LifeState::Alive, Aiming::new(false), affordable));

    let reply: ActReply = decoded(
        ACT_FIRE,
        ask_while_setting(
            &mut app,
            &tx,
            ModeKindNet::Full,
            run_request(ACT_FIRE, &cell_argument(at)),
        ),
    );

    assert_eq!(
        reply,
        ActReply::FireRefused {
            reason: ShotRefusalNet::Unaffordable,
        },
        "the shooter was left holding exactly what the gun's single costs, so a shot built with \
         the full-auto mode set on this frame cannot be paid for; anything else means the act \
         priced the mode the write replaced",
    );
}

#[test]
fn the_fire_target_highlight_prices_the_mode_set_on_its_own_frame() {
    let (mut app, tx) = battle_with_a_gun(single_burst_and_full());
    let Some((_enemy, at)) = an_enemy_ganger_at(&app) else {
        unreachable!("the fixture fields an enemy for the shooter to be offered a shot at");
    };
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(at.to_sim())));
    let shooter = selected_shooter(&app);
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    let priced_full = shot_price(&app, &FULL, at.to_sim());
    let priced_single = shot_price(&app, &SINGLE, at.to_sim());
    assert_ne!(
        priced_full, priced_single,
        "the two modes have to cost the shooter different amounts, or the highlight cannot say \
         which one it was priced with",
    );

    let _answer = run_one_frame(
        &mut app,
        &tx,
        BATTLE_SET_FIRE_MODE,
        &mode_argument(ModeKindNet::Full),
    );

    let highlight = app.world().get_resource::<FireTargetHighlight>().copied();
    assert_eq!(
        highlight.and_then(|drawn| drawn.cost()),
        Some(priced_full),
        "the overlay prices the hovered enemy with the live mode, so the single's price here \
         means it was drawn before the write that set full landed: {highlight:?}",
    );
}
