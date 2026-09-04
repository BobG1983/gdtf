//! Who `battle.cost` will price an act for, and who it turns down unpriced.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::message::McpRequest;
use gdtf_game::qa_wire::{
    act_payload::StanceNet,
    cost::{CostActNet, CostRefusalNet},
    token::GangerToken,
};

use super::support::{cost_body, cost_call, cost_calls, settle};
use crate::{
    battle_reads::an_enemy_ganger, command_exchange::exchange_in_battle, socket_support::TestResult,
};

/// A token no ganger in any battle answers to.
const NOT_A_GANGER: GangerToken = GangerToken::new(u64::MAX);

/// Acts asked of a ganger the player does not command, then of a token naming nothing.
///
/// `SetStance` is priced from tuning and the actor's own row alone, so it is the one act that
/// would come back with a number if the scope check were gone.
fn out_of_scope_calls(enemy: Entity) -> Vec<McpRequest> {
    let mut calls = cost_calls(
        enemy,
        &[
            CostActNet::SetStance {
                stance: StanceNet::Prone,
            },
            CostActNet::Reload,
        ],
    );
    calls.extend(cost_call(
        NOT_A_GANGER,
        &CostActNet::SetStance {
            stance: StanceNet::Prone,
        },
    ));
    calls
}

#[test]
fn a_ganger_the_player_does_not_command_is_refused_and_never_priced() -> TestResult {
    let mut enemy: Option<Entity> = None;
    let (_app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some(found) = an_enemy_ganger(app) else {
            return Vec::new();
        };
        enemy = Some(found);
        out_of_scope_calls(found)
    })?;
    if enemy.is_none() {
        return Err("a running battle must hold a living enemy ganger to ask about".into());
    }

    let refusals = [
        CostRefusalNet::NotYourGanger,
        CostRefusalNet::NotYourGanger,
        CostRefusalNet::NoSuchGanger,
    ];
    assert_eq!(
        replies.len(),
        refusals.len(),
        "every act asked must come back with its own reply",
    );
    for (reply, expected) in replies.into_iter().zip(refusals) {
        let body = cost_body(reply)?;
        assert_eq!(
            body.refusal,
            Some(expected),
            "a ganger outside the player's command is refused {expected:?}: {body:?}",
        );
        assert!(!*body.legal, "a refused act is not legal: {body:?}");
        assert_eq!(
            body.cost, None,
            "an act the command will not price carries no number at all: {body:?}",
        );
    }
    Ok(())
}
