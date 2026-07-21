//! The query-sourced half of the CONSEQUENCE family — the settled VITALS and MAGAZINE
//! after-values no sim message owns (GTW-727 C4 / C9 family 5, C11).

use bevy::ecs::relationship::Relationship;

use super::super::sources::{ConsequenceState, ProvenanceSources, cell_order};
use crate::act_log::{
    ActDeed, ActLog, RecordedAct,
    facts::{MagazineFacts, VitalsFacts},
};

/// Record every ganger whose VITALS settled to new values this tick.
///
/// Vitals are query-sourced rather than message-sourced because damage arrives through
/// paths that share no signal: a shot's in-fold apply, a melee strike, a fall, and the
/// bleed / DOT / field per-round clocks all mutate HP and wounds directly. One transition
/// recorder carries the settled numbers from every one of them, so a consumer that mirrors
/// the stat block applies exactly what the sim reached — in log order, AFTER the deed that
/// caused it — instead of reading live state and showing a wound before its bolt lands.
///
/// Obeys both halves of C11: transitions only (a first observation seeds the prior-value
/// map and records nothing, so a roster spawn records no vitals), and a deterministic
/// `(level, y, x)` visit order.
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

/// Record every wielded weapon whose MAGAZINE settled to a new load this tick.
///
/// The entry's actor is the WEAPON entity — the same entity the ammo readout resolves
/// through `ganger → Wields → weapon` — so a consumer applies the recorded load to exactly
/// what it renders. Weapons are visited in their WIELDER's `(level, y, x)` order, so the
/// pass orders by the same game-state key the ganger recorders use rather than by entity
/// index; a weapon whose wielder has no resolvable position sorts last, deterministically.
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
