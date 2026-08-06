//! Aiming and facing over a real socket: an absolute value, not a cycle.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    act::ActSeqNet,
    act_payload::{AimNet, FacingNet},
    deed::ActDeedKindNet,
    token::GangerToken,
};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{
        LogBody, accepted, battle_app_reporting, caught_up, decode, next, selected, window,
    },
    battle_reads::{a_player_ganger, aiming_of, facing_of, ganger_argument, token_of},
    command_exchange::{
        ACT_SELECT, ACT_SET_AIMING, ACT_SET_FACING, LOG_READ, WAIT,
        assert_refused_off_the_battle_screen, exchange_inspecting, run,
    },
    socket_support::{TestResult, game_app_listening},
};

/// The compass ring, in the order the sim turns through it.
const COMPASS: [FacingNet; 8] = [
    FacingNet::North,
    FacingNet::NorthEast,
    FacingNet::East,
    FacingNet::SouthEast,
    FacingNet::South,
    FacingNet::SouthWest,
    FacingNet::West,
    FacingNet::NorthWest,
];

/// Why the fixture cannot host a case that needs a ganger of the player's.
const NO_GANGER: &str = "the fixture must hold a player ganger to give orders to";

/// The ganger a posture case drives, and where it was pointing to begin with.
struct Poser {
    ganger: Entity,
    facing: FacingNet,
}

/// The direction `steps` further round the ring than `from`.
fn round_from(from: FacingNet, steps: usize) -> FacingNet {
    let Some(at) = COMPASS.iter().position(|point| *point == from) else {
        unreachable!("every wire facing is on the compass ring");
    };
    COMPASS[(at + steps) % COMPASS.len()]
}

fn facing_argument(facing: FacingNet) -> String {
    let Ok(text) = ron::ser::to_string(&facing) else {
        unreachable!("a wire facing serializes to compact RON");
    };
    format!("(facing:{text})")
}

/// Whether the actor's pose changed inside the window one call opened.
fn posture_changed(log: &LogBody, actor: GangerToken, window: (ActSeqNet, ActSeqNet)) -> bool {
    log.deeds_by(actor, window)
        .contains(&ActDeedKindNet::PostureChanged)
}

#[test]
fn aiming_is_the_value_asked_for_rather_than_a_toggle() -> TestResult {
    let (app, replies, poser) = exchange_inspecting(
        battle_app_reporting(
            |app| {
                let (ganger, _) = a_player_ganger(app)?;
                Some(Poser {
                    ganger,
                    facing: facing_of(app, ganger)?,
                })
            },
            NO_GANGER,
        ),
        |poser| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(poser.ganger),
                    RunOptions::default(),
                ),
                run(ACT_SET_AIMING, "(aim:false)", RunOptions::default()),
                caught_up(),
                run(ACT_SET_AIMING, "(aim:true)", RunOptions::default()),
                caught_up(),
                run(ACT_SET_AIMING, "(aim:true)", RunOptions::default()),
                caught_up(),
                run(ACT_SET_AIMING, "(aim:false)", RunOptions::default()),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _selected = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let _lowered = accepted(ACT_SET_AIMING, next(ACT_SET_AIMING, &mut replies)?)?;
    let _first = next(WAIT, &mut replies)?;
    let raised = accepted(ACT_SET_AIMING, next(ACT_SET_AIMING, &mut replies)?)?;
    let _second = next(WAIT, &mut replies)?;
    let again = accepted(ACT_SET_AIMING, next(ACT_SET_AIMING, &mut replies)?)?;
    let _third = next(WAIT, &mut replies)?;
    let lowered = accepted(ACT_SET_AIMING, next(ACT_SET_AIMING, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let actor = token_of(poser.ganger);
    let (Some(raised), Some(again), Some(lowered)) =
        (window(raised), window(again), window(lowered))
    else {
        unreachable!("every accepted act carries its window");
    };
    assert!(
        posture_changed(&log, actor, raised),
        "taking aim changes the pose the sim holds, so the frame must log it: {raised:?} in {:?}",
        log.entries,
    );
    assert!(
        !posture_changed(&log, actor, again),
        "asking for the aim already held changes nothing, so that frame logs no pose at all — a \
         toggle would have logged one: {again:?} in {:?}",
        log.entries,
    );
    assert!(
        posture_changed(&log, actor, lowered),
        "lowering the weapon is the value asked for, so it reaches the sim too: {lowered:?} in \
         {:?}",
        log.entries,
    );
    assert_eq!(
        aiming_of(&app, poser.ganger),
        Some(AimNet::new(false)),
        "the last call asked for `false`, so that is what the ganger must be holding — read off \
         the sim's own aim flag, which is the only thing that says the value asked for was not \
         inverted on its way through",
    );
    Ok(())
}

#[test]
fn facing_turns_to_the_direction_named_rather_than_cycling() -> TestResult {
    let (app, replies, poser) = exchange_inspecting(
        battle_app_reporting(
            |app| {
                let (ganger, _) = a_player_ganger(app)?;
                Some(Poser {
                    ganger,
                    facing: facing_of(app, ganger)?,
                })
            },
            NO_GANGER,
        ),
        |poser| {
            let quarter = round_from(poser.facing, 2);
            let half = round_from(poser.facing, 4);
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(poser.ganger),
                    RunOptions::default(),
                ),
                run(
                    ACT_SET_FACING,
                    &facing_argument(quarter),
                    RunOptions::default(),
                ),
                caught_up(),
                run(
                    ACT_SET_FACING,
                    &facing_argument(quarter),
                    RunOptions::default(),
                ),
                caught_up(),
                run(
                    ACT_SET_FACING,
                    &facing_argument(half),
                    RunOptions::default(),
                ),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _selected = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let turned = accepted(ACT_SET_FACING, next(ACT_SET_FACING, &mut replies)?)?;
    let _first = next(WAIT, &mut replies)?;
    let again = accepted(ACT_SET_FACING, next(ACT_SET_FACING, &mut replies)?)?;
    let _second = next(WAIT, &mut replies)?;
    let further = accepted(ACT_SET_FACING, next(ACT_SET_FACING, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let actor = token_of(poser.ganger);
    let (Some(turned), Some(again), Some(further)) =
        (window(turned), window(again), window(further))
    else {
        unreachable!("every accepted act carries its window");
    };
    assert!(
        posture_changed(&log, actor, turned),
        "turning a quarter of the ring changes the pose, so the frame must log it: {turned:?} in \
         {:?}",
        log.entries,
    );
    assert!(
        !posture_changed(&log, actor, again),
        "the same direction twice is idempotent, so that frame logs no pose at all: {again:?} in \
         {:?}",
        log.entries,
    );
    assert!(
        posture_changed(&log, actor, further),
        "a second, different direction has to reach the sim as the value asked for — a fixed \
         direction would already be standing there: {further:?} in {:?}",
        log.entries,
    );
    assert_eq!(
        facing_of(&app, poser.ganger),
        Some(round_from(poser.facing, 4)),
        "the last call named a direction, so the ganger must be standing in it — read off the \
         sim's own facing, which is the only thing that says the direction asked for is the \
         direction reached",
    );
    Ok(())
}

#[test]
fn aiming_and_facing_refuse_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SET_AIMING, "(aim:true)")?;
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SET_FACING, "(facing:North)")?;
    Ok(())
}
