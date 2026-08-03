use bevy::{platform::collections::HashSet, prelude::Entity};

use super::sources::{MovementSources, ProvenanceSources, cell_order};
use crate::act_log::{ActDeed, ActLog, RecordedAct, facts::PositionFacts};

pub(super) fn record_movement(
    log: &mut ActLog,
    movement: &mut MovementSources,
    provenance: &ProvenanceSources,
) {
    let mut stepped: HashSet<Entity> = HashSet::default();
    for step in movement.steps.read() {
        let Ok((_, position)) = movement.positions.get(step.actor) else {
            continue;
        };
        stepped.insert(step.actor);
        log.append(RecordedAct::new(
            step.actor,
            provenance.of(step.actor),
            ActDeed::Stepped {
                from:     step.from,
                to:       step.to,
                position: PositionFacts::new(*position),
            },
        ));
    }
    for refusal in movement.refusals.read() {
        log.append(RecordedAct::new(
            refusal.actor,
            provenance.of(refusal.actor),
            ActDeed::MoveRefused {
                reason: refusal.reason,
            },
        ));
    }

    let mut rows: Vec<_> = movement
        .positions
        .iter()
        .map(|(entity, position)| (cell_order(position), entity, PositionFacts::new(*position)))
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);
    for (_, entity, position) in rows {
        if log.note_position(entity, position) && !stepped.contains(&entity) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::MovedTo { position },
            ));
        }
    }
}
