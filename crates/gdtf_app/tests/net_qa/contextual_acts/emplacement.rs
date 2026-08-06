//! The emplacement pair over a real socket: mounting and dismounting what the panel offers.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{offer::OfferTargetNet, token::EmplacementToken};
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    emplacement::{EmplacementOccupant, EmplacementState, SetEmplacement},
    entity::TerrainCell,
    ganger::Position,
    prelude::CellLevel,
};
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};
use gdtf_test_utils::advance_until;

use super::{
    super::{
        act_support::{caught_up, next},
        battle_reads::clear_cells_away_from,
        command_exchange::{ACT_ENTER_EMPLACEMENT, ACT_EXIT_EMPLACEMENT, exchange_inspecting, run},
        socket_support::{TestError, TestResult, battle_app_listening},
    },
    scene::{SETTLE_BUDGET, a_neighbour, accepted, select_a_player_ganger, settle},
};

/// Who is manning an emplacement right now.
fn occupant_of(app: &App, emplacement: Entity) -> Option<Entity> {
    app.world()
        .get::<EmplacementOccupant>(emplacement)
        .map(|occupant| **occupant)
}

/// Whether an emplacement is manned right now.
fn state_of(app: &App, emplacement: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(emplacement).copied()
}

/// Every vacant emplacement the generated map already put 8-adjacent to `at`.
fn vacant_emplacements_around(app: &App, at: CellLevel) -> Vec<Entity> {
    app.world()
        .iter_entities()
        .filter_map(|entity| {
            let state = *entity.get::<EmplacementState>()?;
            let cell = **entity.get::<TerrainCell>()?;
            (!*state.is_occupied() && *is_8_adjacent(Position::new(at), Position::new(cell)))
                .then_some(entity.id())
        })
        .collect()
}

/// Move the map's own vacant emplacements out of reach, so only ours can be offered.
fn clear_emplacements_around(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let crowd = vacant_emplacements_around(app, at);
    let mut away = clear_cells_away_from(app, at).into_iter();
    for emplacement in crowd {
        let Some(cell) = away.next() else {
            return Err(
                "the map must hold a clear cell out of reach for each emplacement it \
                        started beside the shooter"
                    .into(),
            );
        };
        let Ok(mut row) = app.world_mut().get_entity_mut(emplacement) else {
            return Err("the emplacement the world just answered with must still exist".into());
        };
        row.insert(TerrainCell::new(cell));
    }
    Ok(())
}

/// A live battle whose selected shooter has exactly one vacant emplacement beside it: ours.
fn emplacement_beside_the_shooter() -> Result<(App, NetQaPort, (Entity, Entity)), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (shooter, at) = select_a_player_ganger(&mut app)?;
    clear_emplacements_around(&mut app, at)?;
    let beside = a_neighbour(&app, at)?;
    let emplacement = app
        .world_mut()
        .spawn((TerrainCell::new(beside), EmplacementState::Vacant))
        .id();
    settle(&mut app);
    Ok((app, port, (shooter, emplacement)))
}

/// The same battle with the shooter already manning that emplacement, through the sim's toggle.
fn manned_emplacement_under_the_shooter() -> Result<(App, NetQaPort, (Entity, Entity)), TestError> {
    let (mut app, port, (shooter, emplacement)) = emplacement_beside_the_shooter()?;
    app.world_mut()
        .write_message(SetEmplacement::occupy(emplacement, shooter));
    if advance_until(
        &mut app,
        |app| occupant_of(app, emplacement) == Some(shooter),
        SETTLE_BUDGET,
    ) {
        return Ok((app, port, (shooter, emplacement)));
    }
    Err(format!(
        "the sim's own toggle must man the emplacement within {SETTLE_BUDGET} frames, or the \
         dismount case has nothing to dismount"
    )
    .into())
}

#[test]
fn entering_the_offered_emplacement_names_it_and_mans_it_in_the_world() -> TestResult {
    let (mut app, replies, (shooter, emplacement)) =
        exchange_inspecting(emplacement_beside_the_shooter, |_placed| {
            vec![
                caught_up(),
                run(ACT_ENTER_EMPLACEMENT, "()", RunOptions::default()),
            ]
        })?;
    let mut replies = replies.into_iter();
    let _caught = next("wait", &mut replies)?;
    let entered = accepted(
        ACT_ENTER_EMPLACEMENT,
        next(ACT_ENTER_EMPLACEMENT, &mut replies)?,
    )?;

    assert_eq!(
        entered.target,
        OfferTargetNet::Emplacement(EmplacementToken::new(emplacement.to_bits())),
        "the reply names the emplacement the panel was offering, which is the one the call \
         fired at",
    );
    let manned = advance_until(
        &mut app,
        |app| occupant_of(app, emplacement) == Some(shooter),
        SETTLE_BUDGET,
    );
    assert!(
        manned,
        "the sim's own toggle must leave the shooter manning the carried emplacement; the act \
         log holds no deed for one, so the world is the evidence — it was {:?} with occupant {:?}",
        state_of(&app, emplacement),
        occupant_of(&app, emplacement),
    );
    Ok(())
}

#[test]
fn exiting_the_manned_emplacement_names_it_and_leaves_it_vacant() -> TestResult {
    let (mut app, replies, (_shooter, emplacement)) =
        exchange_inspecting(manned_emplacement_under_the_shooter, |_manned| {
            vec![
                caught_up(),
                run(ACT_EXIT_EMPLACEMENT, "()", RunOptions::default()),
            ]
        })?;
    let mut replies = replies.into_iter();
    let _caught = next("wait", &mut replies)?;
    let exited = accepted(
        ACT_EXIT_EMPLACEMENT,
        next(ACT_EXIT_EMPLACEMENT, &mut replies)?,
    )?;

    assert_eq!(
        exited.target,
        OfferTargetNet::Emplacement(EmplacementToken::new(emplacement.to_bits())),
        "the reply names the emplacement the shooter was riding, which is the one it left",
    );
    let vacated = advance_until(
        &mut app,
        |app| state_of(app, emplacement) == Some(EmplacementState::Vacant),
        SETTLE_BUDGET,
    );
    assert!(
        vacated,
        "the sim's own toggle must leave the carried emplacement vacant; it was {:?} with \
         occupant {:?}",
        state_of(&app, emplacement),
        occupant_of(&app, emplacement),
    );
    assert_eq!(
        occupant_of(&app, emplacement),
        None,
        "dismounting clears the occupant; a command wired to the enter family would have \
         re-manned it instead",
    );
    Ok(())
}
