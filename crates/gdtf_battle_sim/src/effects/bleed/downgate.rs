//! The **damage down-gate** marker insertion — [`mark_downed_bleeding`] reifies the
//! [`crate::apply_hit`] `LifeState::Downed` write as a [`BleedingOut`] condition (GTW-695).

use bevy::prelude::{Changed, Commands, Entity, Query, Without};

use super::tick::BleedingOut;
use crate::ganger::LifeState;

/// The [`mark_downed_bleeding`] query row — a ganger's [`Entity`] + [`LifeState`], filtered
/// to those whose life state CHANGED this frame and that do NOT already carry the
/// [`BleedingOut`] condition. A named alias so the system signature stays under clippy's
/// `type_complexity` gate (the [`crate::acts::downed`] `DownedReads` precedent).
type NewlyDowned<'world, 'state> =
    Query<'world, 'state, (Entity, &'static LifeState), (Changed<LifeState>, Without<BleedingOut>)>;

/// Insert the [`BleedingOut`] condition on every ganger the DAMAGE pipeline just downed —
/// the [`crate::apply_hit`] down-gate site of the two GTW-695 down-transition sites (the
/// other, [`tick_bleed`](crate::effects::bleed::tick_bleed)'s own injury-HP-bleed down-gate,
/// inserts the marker inline).
///
/// Why a change-detection system and not an inline insert at the write: the
/// `apply_hit` fold (`resolution.md` §6 terminal gates) that writes
/// [`LifeState::Downed`] is a PURE, render-free function reached deep through the fire /
/// melee / fall pipelines, none of which carry [`Commands`] — the whole damage pipeline
/// mutates via `Query`/`&mut` borrows and communicates via buffered messages, never
/// `Commands` (`bevy-traps.md` #7). Threading a `Commands` handle down through
/// `synthesize_wound` / the melee strike / every intervening fold would pollute the
/// deterministic damage math with a side-effecting param. Instead this system reacts to
/// the transition the fold applies in place: a `Changed<LifeState>` that now reads
/// [`LifeState::Downed`] on a ganger not already [`BleedingOut`] is exactly a fresh
/// damage-driven down, so it inserts the marker.
///
/// The `Without<BleedingOut>` filter keeps it from touching gangers that already carry
/// the condition (including one [`tick_bleed`](super::tick_bleed) downed and marked this
/// same frame — its inline deferred insert has applied by the time this runs
/// `.after(tick_bleed)`), so the two sites never double-handle. It writes only via
/// [`Commands`] (deferred), so the marker is visible to the NEXT
/// [`tick_bleed`](super::tick_bleed) round — a ganger downed by damage during a
/// turn bleeds from the following enemy-phase tick, exactly as the flag model's
/// `LifeState::Downed` read did. Param-only, no `&mut World` (`bevy-traps.md` #7).
pub fn mark_downed_bleeding(newly_downed: NewlyDowned, mut commands: Commands) {
    for (entity, life) in &newly_downed {
        if *life == LifeState::Downed {
            commands.entity(entity).insert(BleedingOut);
        }
    }
}
