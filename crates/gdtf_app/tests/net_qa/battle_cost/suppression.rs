//! A suppressed mover is refused `Suppressed`, not quoted a walk the sim would turn down.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
};
use gdtf_battle_sim::{
    ganger::{Position, Suppressed, SuppressorCell},
    prelude::{Cell, CellLevel},
};

use super::support::{a_reachable_cell, cost_calls, first_body, settle};
use crate::{
    battle_reads::a_player_ganger,
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// What the case lined up: the mover, and the routable cell it asked to walk to.
struct Pinned {
    actor: Entity,
    dest:  CellLevel,
}

/// A cell past `dest` along the way there, so `dest` is no farther from it than `start`.
///
/// The gate then refuses on geometry alone, whatever the generated map put cover on.
fn beyond(start: CellLevel, dest: CellLevel) -> CellLevel {
    let (from, to) = (start.cell(), dest.cell());
    CellLevel::new(
        Cell::new(to.x + (to.x - from.x), to.y + (to.y - from.y)),
        dest.level(),
    )
}

/// Suppress the actor from a cell the walk to `dest` cannot break away from.
fn pin(app: &mut App, actor: Entity, dest: CellLevel) -> Option<Pinned> {
    let start = **app.world().get_entity(actor).ok()?.get::<Position>()?;
    app.world_mut()
        .get_entity_mut(actor)
        .ok()?
        .insert(Suppressed::new(SuppressorCell::new(beyond(start, dest))));
    Some(Pinned { actor, dest })
}

#[test]
fn a_suppressed_mover_is_refused_suppressed_rather_than_quoted_a_legal_walk() -> TestResult {
    let mut planned: Option<Pinned> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, _at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        let Some(dest) = a_reachable_cell(app, actor) else {
            return Vec::new();
        };
        let Some(pinned) = pin(app, actor, dest) else {
            return Vec::new();
        };
        let calls = cost_calls(
            pinned.actor,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(pinned.dest),
            }],
        );
        planned = Some(pinned);
        calls
    })?;
    let Some(pinned) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with a cell it can walk to",
        ));
    };
    assert!(
        app.world()
            .get_entity(pinned.actor)
            .is_ok_and(|row| row.contains::<Suppressed>()),
        "the mover must still be suppressed when the reply landed, or the case proves nothing",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::Suppressed),
        "a walk the suppression gate turns down must be refused Suppressed, not priced legal: \
         {body:?}",
    );
    assert!(
        !*body.legal,
        "a suppressed mover's walk is not legal: {body:?}",
    );
    Ok(())
}
