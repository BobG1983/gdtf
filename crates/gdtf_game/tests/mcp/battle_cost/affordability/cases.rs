//! `battle.cost` says `CannotAfford` and the sim's own dispatch refuses the very same act.

use gdtf_battle_sim::{
    acts::{MeleeRequested, SetStanceRequested, ShoveRequested},
    ganger::Stance,
};
use gdtf_game::qa_wire::{
    act_payload::{MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    token::GangerToken,
};

use super::fixture::{
    cell_of, other_stance, pool_of, set_pool, stance_of, too_poor_to_change_stance,
    too_poor_to_shove, too_poor_to_smash, vitals_of,
};
use crate::{
    battle_cost::support::{cost_calls, first_body, settle},
    command_exchange::exchange_inspecting,
    socket_support::TestResult,
};

#[test]
fn an_unaffordable_stance_change_is_refused_and_the_sim_changes_nothing() -> TestResult {
    let (mut app, replies, (broke, held)) =
        exchange_inspecting(too_poor_to_change_stance, |(broke, held)| {
            cost_calls(
                broke.actor,
                &[CostActNet::SetStance {
                    stance: StanceNet::from_sim(Stance::new(other_stance(*held))),
                }],
            )
        })?;
    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::CannotAfford),
        "a stance change the pool cannot cover must be refused CannotAfford: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");

    let asked = other_stance(held);
    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "the actor must still be short of the cost when the reply landed, or the case proves \
         nothing",
    );
    app.world_mut()
        .write_message(SetStanceRequested::new(broke.actor, asked));
    settle(&mut app);
    assert_eq!(
        stance_of(&app, broke.actor),
        Some(held),
        "the sim must answer the same way the quote did: the stance does not change",
    );
    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "a stance change the sim refused spends nothing",
    );

    set_pool(&mut app, broke.actor, broke.cost)?;
    app.world_mut()
        .write_message(SetStanceRequested::new(broke.actor, asked));
    settle(&mut app);
    assert_eq!(
        stance_of(&app, broke.actor),
        Some(asked),
        "the same request from a pool that covers the cost does change the stance",
    );
    Ok(())
}

#[test]
fn an_unaffordable_shove_is_refused_and_the_sim_pushes_nobody() -> TestResult {
    let (mut app, replies, (broke, enemy)) =
        exchange_inspecting(too_poor_to_shove, |(broke, enemy)| {
            cost_calls(
                broke.actor,
                &[CostActNet::Shove {
                    target: GangerToken::new(enemy.to_bits()),
                }],
            )
        })?;
    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::CannotAfford),
        "a shove the pool cannot cover must be refused CannotAfford: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");

    let (stood, hurt) = (cell_of(&app, enemy), vitals_of(&app, enemy));
    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "the actor must still be short of the cost when the reply landed, or the case proves \
         nothing",
    );
    app.world_mut()
        .write_message(ShoveRequested::new(broke.actor, enemy));
    settle(&mut app);
    assert_eq!(
        cell_of(&app, enemy),
        stood,
        "the sim must answer the same way the quote did: the target is not pushed",
    );
    assert_eq!(
        vitals_of(&app, enemy),
        hurt,
        "a shove the sim refused does the target no harm",
    );
    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "a shove the sim refused spends nothing",
    );

    set_pool(&mut app, broke.actor, broke.cost)?;
    app.world_mut()
        .write_message(ShoveRequested::new(broke.actor, enemy));
    settle(&mut app);
    assert_ne!(
        cell_of(&app, enemy),
        stood,
        "the same shove from a pool that covers the cost does push the target",
    );
    Ok(())
}

#[test]
fn an_unaffordable_strike_is_refused_and_the_sim_swings_at_nothing() -> TestResult {
    let (mut app, replies, (broke, cell)) =
        exchange_inspecting(too_poor_to_smash, |(broke, cell)| {
            cost_calls(
                broke.actor,
                &[CostActNet::Melee {
                    target: MeleeTargetNet::Structure(CellLevelNet::from_sim(*cell)),
                }],
            )
        })?;
    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::CannotAfford),
        "a strike the pool cannot cover must be refused CannotAfford: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");

    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "the actor must still be short of the cost when the reply landed, or the case proves \
         nothing",
    );
    app.world_mut()
        .write_message(MeleeRequested::new_structural(broke.actor, cell));
    settle(&mut app);
    assert_eq!(
        pool_of(&app, broke.actor),
        Some(broke.pool),
        "the sim must answer the same way the quote did: a strike it refused spends nothing",
    );

    set_pool(&mut app, broke.actor, broke.cost)?;
    app.world_mut()
        .write_message(MeleeRequested::new_structural(broke.actor, cell));
    settle(&mut app);
    assert!(
        pool_of(&app, broke.actor).is_some_and(|left| *left < *broke.cost),
        "the same strike from a pool that covers the cost is charged for",
    );
    Ok(())
}
