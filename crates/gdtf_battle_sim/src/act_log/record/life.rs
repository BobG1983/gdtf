//! The LIFE-family recorder — the genuine `from → to` life-state transition (GTW-727 C9
//! family 6, C11).

use super::sources::{LifeSources, ProvenanceSources, cell_order};
use crate::act_log::{ActDeed, ActLog, RecordedAct};

/// Record every ganger whose LIFE STATE transitioned this tick.
///
/// The LAST family in the fixed recording order, so a killing round's own entry — and the
/// vitals that round left behind — always precede the death it caused. A consumer replaying
/// the log therefore shows the shot, then the damage, then the body: never the reverse.
///
/// This is the only place in the sim that produces a life-state TRANSITION rather than a
/// post-act snapshot. Nothing else carries the state the ganger LEFT: a `Downed` fact has
/// no dedicated sim message at all, which is why every downstream consumer has had to run
/// its own change detection to recover the edge.
///
/// Obeys both halves of C11: a first observation seeds the prior-value map and records
/// nothing (so setting every ganger's life state at spawn records no transitions), and
/// gangers are visited in `(level, y, x)` order.
pub(super) fn record_life(log: &mut ActLog, lives: &LifeSources, provenance: &ProvenanceSources) {
    let mut rows: Vec<_> = lives
        .gangers
        .iter()
        .map(|(entity, position, life)| (cell_order(position), entity, *life))
        .collect();
    rows.sort_unstable_by_key(|(order, ..)| *order);

    for (_, entity, life) in rows {
        if let Some(from) = log.note_life(entity, life) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::LifeChanged { from, to: life },
            ));
        }
    }
}
