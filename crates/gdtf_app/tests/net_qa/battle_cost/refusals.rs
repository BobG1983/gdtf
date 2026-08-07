//! The refusals only the live map answers: a cell with no route, and an act the sim rejects.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    act_payload::MeleeTargetNet,
    cost::{CostActNet, CostRefusalNet},
    token::GangerToken,
};

use super::support::{cost_body, cost_calls, first_body, settle};
use crate::{
    battle_reads::{a_player_ganger, an_unreachable_cell},
    command_exchange::exchange_in_battle,
    socket_support::TestResult,
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
