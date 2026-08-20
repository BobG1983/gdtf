use super::sources::{ActObservation, LifeSources, ProvenanceSources, cell_order};
use crate::act_log::{ActDeed, ActLog, PositionFacts, RecordedAct};

pub(super) fn record_life(
    log: &mut ActLog,
    lives: &LifeSources,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
) {
    let mut rows: Vec<_> = lives
        .gangers
        .iter()
        .map(|(entity, position, life)| {
            (
                cell_order(position),
                entity,
                PositionFacts::new(*position),
                *life,
            )
        })
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, entity, at, life) in rows {
        if let Some(from) = log.note_life(entity, life) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::LifeChanged { from, to: life, at },
                seen.of_cell(**at),
            ));
        }
    }
}
