//! Driving the pair over the socket: the reply names the seat, and the card says who is in it.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    act::ActRefusalNet,
    cell::CellLevelNet,
    inspect::InspectShownNet,
    offer::OfferTargetNet,
    roster::{GangerCardNet, MountedNet},
    token::{EmplacementToken, GangerToken},
};
use gdtf_battle_sim::{emplacement::EmplacementState, prelude::CellLevel};
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use gdtf_test_utils::advance_until;
use serde::Deserialize;

use super::fixture::{
    a_held_entry_cell_under_the_manned_emplacement, emplacement_beside_the_shooter,
    manned_emplacement_under_the_shooter, occupant_of,
};
use crate::{
    act_support::{assert_caught_up, caught_up, decode, next},
    battle_reads::cell_argument,
    command_exchange::{
        ACT_ENTER_EMPLACEMENT, ACT_EXIT_EMPLACEMENT, BATTLE_INSPECT, WAIT, exchange_inspecting, run,
    },
    contextual_acts::scene::{accepted, refused},
    socket_support::{TestError, TestResult},
};

/// Whether an emplacement is manned right now.
fn state_of(app: &App, emplacement: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(emplacement).copied()
}

/// The part of a `battle.inspect` reply these cases read.
#[derive(Debug, Deserialize)]
struct InspectBody {
    shown: InspectShownNet,
}

/// The card the panel drew for one cell, or a failure naming what it showed instead.
fn inspected_card(reply: QaResponse) -> Result<GangerCardNet, TestError> {
    let shown = decode::<InspectBody>(BATTLE_INSPECT, reply)?.shown;
    match shown.ganger.clone() {
        Some(card) => Ok(card),
        None => Err(format!(
            "`{BATTLE_INSPECT}` must draw the shooter's card on the cell it is standing on; it \
             showed {shown:?} instead, and a seat with nobody on it reports no ganger half"
        )
        .into()),
    }
}

/// Ask `battle.inspect` about one cell.
fn inspect(at: CellLevel) -> QaRequest {
    run(
        BATTLE_INSPECT,
        &cell_argument(CellLevelNet::from_sim(at)),
        RunOptions::default(),
    )
}

#[test]
fn entering_the_offered_emplacement_names_it_and_mans_it_in_the_world() -> TestResult {
    let (mut app, replies, scene) =
        exchange_inspecting(emplacement_beside_the_shooter, |_placed| {
            vec![
                caught_up(),
                run(ACT_ENTER_EMPLACEMENT, "()", RunOptions::default()),
            ]
        })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let entered = accepted(
        ACT_ENTER_EMPLACEMENT,
        next(ACT_ENTER_EMPLACEMENT, &mut replies)?,
    )?;

    assert_eq!(
        entered.target,
        OfferTargetNet::Emplacement(EmplacementToken::new(scene.emplacement.to_bits())),
        "the reply names the emplacement the panel was offering, which is the one the call \
         fired at",
    );
    advance_until(&mut app, |app| {
        occupant_of(app, scene.emplacement) == Some(scene.shooter)
    });
    Ok(())
}

#[test]
fn exiting_the_manned_emplacement_names_it_and_leaves_it_vacant() -> TestResult {
    let (mut app, replies, manned) =
        exchange_inspecting(manned_emplacement_under_the_shooter, |_manned| {
            vec![
                caught_up(),
                run(ACT_EXIT_EMPLACEMENT, "()", RunOptions::default()),
            ]
        })?;
    let emplacement = manned.emplacement;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let exited = accepted(
        ACT_EXIT_EMPLACEMENT,
        next(ACT_EXIT_EMPLACEMENT, &mut replies)?,
    )?;

    assert_eq!(
        exited.target,
        OfferTargetNet::Emplacement(EmplacementToken::new(emplacement.to_bits())),
        "the reply names the emplacement the shooter was riding, which is the one it left",
    );
    advance_until(&mut app, |app| {
        state_of(app, emplacement) == Some(EmplacementState::Vacant)
    });
    assert_eq!(
        occupant_of(&app, emplacement),
        None,
        "dismounting clears the occupant; a command wired to the enter family would have \
         re-manned it instead",
    );
    Ok(())
}

#[test]
fn the_exit_is_not_offered_while_another_ganger_holds_the_cell_the_shooter_entered_from()
-> TestResult {
    let (_app, replies, _held) =
        exchange_inspecting(a_held_entry_cell_under_the_manned_emplacement, |_held| {
            vec![
                caught_up(),
                run(ACT_EXIT_EMPLACEMENT, "()", RunOptions::default()),
            ]
        })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let reason = refused(
        ACT_EXIT_EMPLACEMENT,
        next(ACT_EXIT_EMPLACEMENT, &mut replies)?,
    )?;

    assert_eq!(
        reason,
        ActRefusalNet::NoOffer,
        "the panel offers no Exit while the cell the shooter entered from holds another ganger, \
         so the command answers the one refusal it has rather than firing at the seat",
    );
    Ok(())
}

#[test]
fn the_inspect_read_says_a_mounted_ganger_is_mounted_and_a_dismounted_one_is_not() -> TestResult {
    let (_app, replies, scene) = exchange_inspecting(emplacement_beside_the_shooter, |scene| {
        vec![
            caught_up(),
            run(ACT_ENTER_EMPLACEMENT, "()", RunOptions::default()),
            caught_up(),
            inspect(scene.seat),
            run(ACT_EXIT_EMPLACEMENT, "()", RunOptions::default()),
            caught_up(),
            inspect(scene.entry),
        ]
    })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    accepted(
        ACT_ENTER_EMPLACEMENT,
        next(ACT_ENTER_EMPLACEMENT, &mut replies)?,
    )?;
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let seated = inspected_card(next(BATTLE_INSPECT, &mut replies)?)?;
    accepted(
        ACT_EXIT_EMPLACEMENT,
        next(ACT_EXIT_EMPLACEMENT, &mut replies)?,
    )?;
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let afoot = inspected_card(next(BATTLE_INSPECT, &mut replies)?)?;

    let token = GangerToken::new(scene.shooter.to_bits());
    assert_eq!(
        seated.token, token,
        "the seat at {:?} draws the shooter's own card after it enters: {seated:?}",
        scene.seat,
    );
    assert_eq!(
        seated.mounted,
        MountedNet::new(true),
        "a ganger riding an emplacement reports mounted, which is the only way a reader can \
         tell it apart from one standing on the same cell: {seated:?}",
    );
    assert_eq!(
        afoot.token, token,
        "the entry cell at {:?} draws the same shooter's card after it leaves: {afoot:?}",
        scene.entry,
    );
    assert_eq!(
        afoot.mounted,
        MountedNet::new(false),
        "leaving clears the marker: {afoot:?}",
    );
    Ok(())
}
