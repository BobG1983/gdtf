use bevy::ecs::relationship::Relationship;

use super::super::sources::{ConsequenceState, ProvenanceSources, cell_order};
use crate::act_log::{
    ActDeed, ActLog, RecordedAct,
    facts::{MagazineFacts, VitalsFacts},
};

pub(in crate::act_log::record) fn record_vitals(
    log: &mut ActLog,
    state: &ConsequenceState,
    provenance: &ProvenanceSources,
) {
    let mut rows: Vec<_> = state
        .vitals
        .iter()
        .map(|(entity, position, tu, hp, wounds, inflicted, injuries)| {
            (
                cell_order(position),
                entity,
                VitalsFacts::new(
                    *tu,
                    *hp,
                    *wounds,
                    inflicted.cloned().unwrap_or_default(),
                    injuries.cloned().unwrap_or_default(),
                ),
            )
        })
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, entity, vitals) in rows {
        if log.note_vitals(entity, &vitals) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::VitalsChanged { vitals },
            ));
        }
    }
}

pub(in crate::act_log::record) fn record_magazines(
    log: &mut ActLog,
    state: &ConsequenceState,
    provenance: &ProvenanceSources,
) {
    let mut rows: Vec<_> = state
        .magazines
        .iter()
        .map(|(weapon, magazine, wielded_by)| {
            let wielder = wielded_by.get();
            let order = state
                .wielders
                .get(wielder)
                .map_or((i32::MAX, i32::MAX, i32::MAX), cell_order);
            (order, weapon, wielder, MagazineFacts::new(*magazine))
        })
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, weapon, wielder, magazine) in rows {
        if log.note_magazine(weapon, magazine) {
            log.append(RecordedAct::new(
                weapon,
                provenance.of(wielder),
                ActDeed::MagazineChanged { magazine },
            ));
        }
    }
}
