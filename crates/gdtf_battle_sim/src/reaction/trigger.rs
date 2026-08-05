//! System: movers and declarers can be interrupted by enemy reactors in LOS.

use bevy::{
    platform::collections::HashSet,
    prelude::{Entity, ResMut},
};

use super::{
    declared::InterruptSignals,
    interrupt::try_reaction,
    ledger::PendingSpendLedger,
    params::{ReactionGrids, ReactionTriggers, ReactorArms, ReactorEligibility},
    snapshot::{ReactionGangers, ReactionPair, ReactionPass, ReactionRow, cell_order},
};
use crate::rng::ReactionRng;

/// On position change or loud fire declaration, try enemy opportunity shots.
pub fn reaction_trigger(
    mut triggers: ReactionTriggers,
    gangers: ReactionGangers,
    arms: ReactorArms,
    mut eligibility: ReactorEligibility,
    grids: ReactionGrids,
    rng: Option<ResMut<ReactionRng>>,
    mut signals: InterruptSignals,
) {
    let Some(mut rng) = rng else {
        return;
    };

    let rows: Vec<ReactionRow> = gangers
        .iter()
        .map(
            |(entity, position, stance, facing, aiming, life, tu, tu_max, faction, reactions)| {
                ReactionRow {
                    entity,
                    position: *position,
                    stance: *stance,
                    facing: *facing,
                    aiming: *aiming,
                    life: *life,
                    tu: *tu,
                    tu_max: *tu_max,
                    faction: *faction,
                    reactions: *reactions,
                }
            },
        )
        .collect();

    let actors: HashSet<Entity> = triggers.actors(&arms);
    if actors.is_empty() {
        return;
    }

    let mut acting_rows: Vec<ReactionRow> = rows
        .iter()
        .copied()
        .filter(|row| actors.contains(&row.entity))
        .collect();
    acting_rows.sort_by_key(|row| cell_order(&row.position));

    let mut ledger = PendingSpendLedger::default();

    for actor in &acting_rows {
        if !*actor.life.is_active() {
            continue;
        }
        let mut reactors: Vec<ReactionRow> = rows
            .iter()
            .copied()
            .filter(|row| row.faction != actor.faction && row.entity != actor.entity)
            .collect();
        reactors.sort_by_key(|row| cell_order(&row.position));
        for reactor in &reactors {
            let pass = ReactionPass::new(&rows, &ledger);
            let commit = try_reaction(
                ReactionPair { actor, reactor },
                &pass,
                &arms,
                &mut eligibility,
                &grids,
                &mut rng,
                &mut signals,
            );
            if let Some(commit) = commit {
                ledger.commit(commit);
            }
        }
    }
}
