use bevy::prelude::Entity;

use super::sources::{ActObservation, ProvenanceSources, TurnSources};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

pub(super) fn record_turn(
    log: &mut ActLog,
    turns: &mut TurnSources,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
) {
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
            seen.everywhere_unnamed(),
        ));
    }
}
