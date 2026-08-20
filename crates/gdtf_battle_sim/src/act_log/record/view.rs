use super::sources::{ActObservation, ProvenanceSources, ViewSources, cell_order};
use crate::{
    act_log::{ActDeed, ActLog, RecordedAct, SquadSees},
    visibility::{FactionRelation, is_ganger_visible},
};

pub(super) fn record_entered_view(
    log: &mut ActLog,
    view: &ViewSources,
    provenance: &ProvenanceSources,
    seen: &ActObservation,
) {
    let Some(player) = seen.player_faction() else {
        return;
    };
    let Some(squad) = seen.squad() else {
        return;
    };

    let mut rows: Vec<_> = view
        .gangers
        .iter()
        .map(|(entity, position, faction)| {
            let relation = if *faction == player {
                FactionRelation::OwnSquad
            } else {
                FactionRelation::Other
            };
            let visible = *is_ganger_visible(squad, position, relation);
            (cell_order(position), entity, **position, visible)
        })
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, entity, at, visible) in rows {
        if log.note_seen(entity, SquadSees::new(visible)) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::EnteredView { at },
                seen.of_cell(at),
            ));
        }
    }
}
