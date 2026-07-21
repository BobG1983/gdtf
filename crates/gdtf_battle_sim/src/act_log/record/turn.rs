//! The TURN-family recorder — the turn boundary (GTW-727 C9, family 1 of 6).

use bevy::prelude::Entity;

use super::sources::{ProvenanceSources, TurnSources};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

/// Record each turn boundary the cycle engine advanced this tick.
///
/// A turn boundary has no acting entity, so the entry's actor is
/// [`Entity::PLACEHOLDER`] — the same "this fact is not about an entity" convention
/// [`OnDeathOccurred::cover`](crate::effects::on_death::OnDeathOccurred::cover) already
/// uses — and its provenance is [`Clock`](crate::act_log::ActProvenance::Clock).
///
/// The FIRST family in the fixed recording order, so a turn's own boundary always precedes
/// the acts taken inside it.
pub(super) fn record_turn(
    log: &mut ActLog,
    turns: &mut TurnSources,
    provenance: &ProvenanceSources,
) {
    // Drained every run whether or not a turn is active, so the reader cursor never backs
    // up (the `forward_turn_started` drain-don't-replay property).
    for turn in turns.turns.read() {
        if !provenance.turn_active() {
            continue;
        }
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ProvenanceSources::clock(),
            ActDeed::TurnBegan {
                now_active: turn.now_active,
            },
        ));
    }
}
