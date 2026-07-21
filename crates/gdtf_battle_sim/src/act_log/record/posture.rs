//! The POSTURE-family recorder — facing / stance / aim / suppression, detected as a
//! TRANSITION against the log's prior-value map (GTW-727 C9 family 2, C11).

use super::sources::{PostureSources, ProvenanceSources, cell_order};
use crate::act_log::{
    ActDeed, ActLog, RecordedAct,
    facts::{PoseFacts, SuppressedNow},
};

/// Record every ganger whose drawn POSTURE settled to a new combination this tick.
///
/// Query-sourced, so it obeys both halves of C11:
///
/// * **Transitions only.** The pose is compared against the log's prior-value map
///   ([`ActLog::note_pose`]); a FIRST observation seeds the map and records nothing. This
///   is what stops a roster spawn from flooding the log — situation setup writes facing,
///   stance and aiming on every ganger at once, so a naive `Changed<T>` recorder would
///   fire for every ganger on the spawn frame.
/// * **Deterministic order.** Gangers are visited in `(level, y, x)`
///   ([`cell_order`]) — never raw query order and never entity index — so the recorded
///   order is a pure function of game state.
///
/// Suppression rides here as a FLAG rather than a component presence, so suppression
/// CLEARING (a component removal, which change detection cannot observe) is an ordinary
/// field transition like any other.
pub(super) fn record_posture(
    log: &mut ActLog,
    postures: &PostureSources,
    provenance: &ProvenanceSources,
) {
    let mut rows: Vec<_> = postures
        .gangers
        .iter()
        .map(|(entity, position, facing, stance, aiming, suppressed)| {
            (
                cell_order(position),
                entity,
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

    for (_, entity, pose) in rows {
        if log.note_pose(entity, pose) {
            log.append(RecordedAct::new(
                entity,
                provenance.of(entity),
                ActDeed::PostureChanged { pose },
            ));
        }
    }
}
