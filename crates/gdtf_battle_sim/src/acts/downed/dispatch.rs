//! Message systems for stabilize and execute.

use bevy::prelude::{Commands, MessageReader, Query, Res};

use crate::{
    acts::{
        downed::{Actor, DownedTarget, execute_downed, stabilize_downed},
        request::{ExecuteDownedRequested, StabilizeDownedRequested},
    },
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
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

/// Process stabilize requests.
pub fn dispatch_stabilize_downed(
    mut requests: MessageReader<StabilizeDownedRequested>,
    gangers: DownedReads,
    tuning: Res<CombatTuning>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        let Ok((&target_pos, &target_life, &target_faction, target_bleeding)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:          target_pos,
            life:         target_life,
            faction:      target_faction,
            bleeding_out: target_bleeding.copied(),
        };
        stabilize_downed(&actor, &target, request.target, &mut commands, &tuning);
    }
}

/// Process execute requests.
pub fn dispatch_execute_downed(
    mut requests: MessageReader<ExecuteDownedRequested>,
    mut gangers: DownedReads,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        let Ok((&target_pos, &target_life, &target_faction, target_bleeding)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:          target_pos,
            life:         target_life,
            faction:      target_faction,
            bleeding_out: target_bleeding.copied(),
        };
        let Ok((_, mut life, ..)) = gangers.get_mut(request.target) else {
            continue;
        };
        execute_downed(&actor, &target, &mut life, &tuning);
    }
}
