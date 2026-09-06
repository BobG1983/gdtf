//! Message systems for stabilize and execute.

use bevy::prelude::{Commands, Entity, MessageReader, Query, Res};

use crate::{
    acts::{
        downed::{
            Actor, DownedTarget, can_execute, can_stabilize, execute_downed, execute_tu_cost,
            stabilize_downed, stabilize_tu_cost,
        },
        pending_state::PendingStates,
        request::{ExecuteDownedRequested, StabilizeDownedRequested},
    },
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position, Tu},
    tu::spend_tu,
    tuning::CombatTuning,
};

type DownedReads<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static mut LifeState,
        &'static Faction,
        Option<&'static BleedingOut>,
    ),
>;

// Both gangers as they stand before anything is charged or written.
fn snapshot_pair(
    gangers: &DownedReads,
    actor: Entity,
    target: Entity,
) -> Option<(Actor, DownedTarget)> {
    let (&actor_pos, &actor_life, &actor_faction, _) = gangers.get(actor).ok()?;
    let (&target_pos, &target_life, &target_faction, target_bleeding) = gangers.get(target).ok()?;
    Some((
        Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        },
        DownedTarget {
            pos:          target_pos,
            life:         target_life,
            faction:      target_faction,
            bleeding_out: target_bleeding.copied(),
        },
    ))
}

/// Process stabilize requests.
/// A repeat request in the same frame reads the bleed this run already stopped, so it pays nothing.
pub fn dispatch_stabilize_downed(
    mut requests: MessageReader<StabilizeDownedRequested>,
    gangers: DownedReads,
    mut pools: Query<&mut Tu>,
    tuning: Res<CombatTuning>,
    mut commands: Commands,
) {
    let mut pending: PendingStates<Option<BleedingOut>> = PendingStates::new();
    for request in requests.read() {
        let Some((actor, mut target)) = snapshot_pair(&gangers, request.actor, request.target)
        else {
            continue;
        };
        target.bleeding_out = pending.state_of(request.target, target.bleeding_out);
        let Ok(&pool) = pools.get(request.actor) else {
            continue;
        };
        if !*can_stabilize(&actor, &target, &pool, &tuning) {
            continue;
        }
        let Ok(mut actor_pool) = pools.get_mut(request.actor) else {
            continue;
        };
        if spend_tu(&mut actor_pool, stabilize_tu_cost(&tuning)).is_err() {
            continue;
        }
        pending.record(request.target, None);
        stabilize_downed(
            &actor,
            &target,
            request.target,
            &mut commands,
            &pool,
            &tuning,
        );
    }
}

/// Process execute requests.
pub fn dispatch_execute_downed(
    mut requests: MessageReader<ExecuteDownedRequested>,
    mut gangers: DownedReads,
    mut pools: Query<&mut Tu>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Some((actor, target)) = snapshot_pair(&gangers, request.actor, request.target) else {
            continue;
        };
        let Ok(&pool) = pools.get(request.actor) else {
            continue;
        };
        if !*can_execute(&actor, &target, &pool, &tuning) {
            continue;
        }
        let Ok(mut actor_pool) = pools.get_mut(request.actor) else {
            continue;
        };
        if spend_tu(&mut actor_pool, execute_tu_cost(&tuning)).is_err() {
            continue;
        }
        let Ok((_, mut life, ..)) = gangers.get_mut(request.target) else {
            continue;
        };
        execute_downed(&actor, &target, &mut life, &pool, &tuning);
    }
}
