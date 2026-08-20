use super::sources::{ActObservation, PostureSources, ProvenanceSources, cell_order};
use crate::act_log::{
    ActDeed, ActLog, RecordedAct,
    facts::{PoseFacts, SuppressedNow},
};

pub(super) fn record_posture(
    log: &mut ActLog,
    postures: &PostureSources,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
) {
    let mut rows: Vec<_> = postures
        .gangers
        .iter()
        .map(|(entity, position, facing, stance, aiming, suppressed)| {
            (
                cell_order(position),
                entity,
                **position,
                PoseFacts::new(
                    *facing,
                    *stance,
                    *aiming,
                    SuppressedNow::new(suppressed.is_some()),
                ),
            )
        })
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, entity, at, pose) in rows {
        if log.note_pose(entity, pose) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::PostureChanged { pose },
                seen.of_cell(at),
            ));
        }
    }
}
