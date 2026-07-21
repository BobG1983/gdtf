//! The MOVEMENT-family recorder — the accepted step, the refused commit, and the settled
//! position (GTW-727 C9 family 3, C11).

use bevy::{platform::collections::HashSet, prelude::Entity};

use super::sources::{MovementSources, ProvenanceSources, cell_order};
use crate::act_log::{ActDeed, ActLog, RecordedAct, facts::PositionFacts};

/// Record each accepted walk step, each refused move commit, and any position that settled
/// somewhere no step announced.
///
/// A step's after-value is the mover's SETTLED [`Position`](crate::ganger::Position), read
/// off the live query: the message carries only a ground-cell pair, which cannot express a
/// cross-storey step, and this recorder runs after the simulation band, so the live
/// position IS the post-step position for the step just announced.
///
/// ## Why there is a position TRANSITION scan as well as the step messages
///
/// Walking is not the only thing that moves a ganger: a fall rewrites its position
/// one-shot, a shove knocks it back a cell, and entering an emplacement relocates it —
/// none of which emits a step message. A consumer mirroring the drawn position off step
/// messages alone would leave those sprites stuck at their old cell indefinitely. The scan
/// is a transition against the log's prior-value map, in the same deterministic
/// `(level, y, x)` order the other query-sourced recorders use.
///
/// Movers that ALREADY announced a step this tick are skipped by the scan (their prior
/// value is still updated), so an ordinary walk costs ONE entry rather than two and does
/// not read as a doubled beat.
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
        // The prior map is updated for a stepped mover too, so its next non-step
        // reposition is measured against where the step actually left it.
        if log.note_position(entity, position) && !stepped.contains(&entity) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::MovedTo { position },
            ));
        }
    }
}
