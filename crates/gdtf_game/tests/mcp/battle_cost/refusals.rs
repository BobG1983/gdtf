//! The refusals only the live map answers: a cell with no route, and an act the sim rejects.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_battle_sim::{
    acts::{enter_emplacement_tu_cost, exit_emplacement_tu_cost},
    emplacement::EmplacementState,
    entity::TerrainCell,
    ganger::Tu,
    tuning::CombatTuning,
};
use gdtf_game::qa_wire::{
    act_payload::MeleeTargetNet,
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    token::{EmplacementToken, GangerToken},
};

use super::support::{cost_body, cost_calls, first_body, settle};
use crate::{
    battle_reads::{a_player_ganger, an_unreachable_cell, one_cardinal_step_from},
    command_exchange::{exchange_in_battle, exchange_inspecting},
    contextual_acts::emplacement::a_held_entry_cell_under_the_manned_emplacement,
    socket_support::{TestError, TestResult},
};

/// Acts the sim's own legality checks turn down: shoving yourself, and striking a cell out of
/// reach. Both are priced first, so a reply carries a number with its refusal.
fn rejected_acts(actor: Entity) -> Vec<CostActNet> {
    vec![
        CostActNet::Shove {
            target: GangerToken::new(actor.to_bits()),
        },
        CostActNet::Melee {
            target: MeleeTargetNet::Structure(an_unreachable_cell()),
        },
    ]
}

/// Stand a sideless emplacement on a clear cardinal cell beside `at`, and name it.
fn a_sideless_seat_beside(app: &mut App, at: CellLevelNet) -> Result<EmplacementToken, TestError> {
    let Some(beside) = one_cardinal_step_from(app, at) else {
        return Err(
            "the generated map must offer one clear CARDINAL cell beside the actor, or a \
             default-sided seat would refuse the enter too and the case would pass either way"
                .into(),
        );
    };
    let seat = app
        .world_mut()
        .spawn((TerrainCell::new(beside.to_sim()), EmplacementState::Vacant))
        .id();
    Ok(EmplacementToken::new(seat.to_bits()))
}

/// The pool the actor holds and what the same world charges for one enter.
fn pool_and_enter_cost(app: &App, actor: Entity) -> Option<(Tu, Tu)> {
    let world = app.world();
    let pool = *world.get_entity(actor).ok()?.get::<Tu>()?;
    let tuning = world.get_resource::<CombatTuning>()?;
    Some((pool, enter_emplacement_tu_cost(tuning)))
}

#[test]
fn an_emplacement_naming_no_entry_side_is_refused_the_enter() -> TestResult {
    let mut planned: Option<Result<Entity, TestError>> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, at)) = a_player_ganger(app) else {
            planned = Some(Err(
                "a running battle must field a ganger the player commands".into(),
            ));
            return Vec::new();
        };
        match a_sideless_seat_beside(app, at) {
            Ok(target) => {
                planned = Some(Ok(actor));
                cost_calls(actor, &[CostActNet::EnterEmplacement { target }])
            }
            Err(fault) => {
                planned = Some(Err(fault));
                Vec::new()
            }
        }
    })?;
    let Some(planned) = planned else {
        return Err("the fixture never reported what it lined up to price".into());
    };
    let actor = planned?;
    let Some((pool, cost)) = pool_and_enter_cost(&app, actor) else {
        return Err("the same world must still hold the actor's pool and the tuning".into());
    };
    assert!(
        *pool >= *cost,
        "the actor's pool must cover the enter cost, or the refusal below is CannotAfford \
         rather than the entry-sides gate",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "an emplacement carrying no entry sides names no cell it can be entered from, so the \
         sim turns the enter down: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");
    Ok(())
}

/// The pool the actor holds and what the same world charges for one exit.
fn pool_and_exit_cost(app: &App, actor: Entity) -> Option<(Tu, Tu)> {
    let world = app.world();
    let pool = *world.get_entity(actor).ok()?.get::<Tu>()?;
    let tuning = world.get_resource::<CombatTuning>()?;
    Some((pool, exit_emplacement_tu_cost(tuning)))
}

#[test]
fn an_exit_onto_a_cell_another_ganger_holds_is_priced_and_then_refused() -> TestResult {
    let (app, replies, held) =
        exchange_inspecting(a_held_entry_cell_under_the_manned_emplacement, |held| {
            cost_calls(
                held.shooter,
                &[CostActNet::ExitEmplacement {
                    target: EmplacementToken::new(held.emplacement.to_bits()),
                }],
            )
        })?;
    let Some((pool, cost)) = pool_and_exit_cost(&app, held.shooter) else {
        return Err("the same world must still hold the shooter's pool and the tuning".into());
    };
    assert!(
        *pool >= *cost,
        "the shooter's pool of {} must cover the {} the exit costs, or the refusal below is \
         CannotAfford rather than the held cell",
        *pool,
        *cost,
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "the cell the shooter entered from holds another ganger, so the sim turns the exit \
         down: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");
    Ok(())
}

#[test]
fn a_cell_no_route_reaches_is_refused_and_never_priced() -> TestResult {
    let mut asked = false;
    let (_app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, _at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        asked = true;
        cost_calls(
            actor,
            &[CostActNet::Move {
                dest: an_unreachable_cell(),
            }],
        )
    })?;
    if !asked {
        return Err("a running battle must field a ganger the player commands".into());
    }
    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::NoPathToCell),
        "a cell the pathfinder cannot route to is refused for the route, not for anything else: \
         {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");
    assert_eq!(
        body.cost, None,
        "a walk with no route has no price at all: {body:?}",
    );
    Ok(())
}

#[test]
fn an_act_the_sim_rejects_is_priced_and_then_refused() -> TestResult {
    let mut planned: Option<Vec<CostActNet>> = None;
    let (_app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, _at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        let acts = rejected_acts(actor);
        let calls = cost_calls(actor, &acts);
        planned = Some(acts);
        calls
    })?;
    let Some(acts) = planned else {
        return Err("a running battle must field a ganger the player commands".into());
    };
    assert_eq!(
        replies.len(),
        acts.len(),
        "every act asked must come back with its own reply",
    );
    for (act, reply) in acts.iter().zip(replies) {
        let body = cost_body(reply)?;
        assert_eq!(
            body.refusal,
            Some(CostRefusalNet::ActNotAllowed),
            "the sim's own legality check must turn {act:?} down: {body:?}",
        );
        assert!(!*body.legal, "a refused act is not legal: {body:?}");
        assert!(
            body.cost.is_some(),
            "the sim priced {act:?} before it refused it, so the quote still carries a number: \
             {body:?}",
        );
    }
    Ok(())
}
