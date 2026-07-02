//! The shared **silenced-weapon gate** — [`shooter_weapon_silenced`], the predicate BOTH
//! loud-signal producers key off so a silenced shot propagates neither SUPPRESSION nor
//! REACTION/REVEAL.
//!
//! The [`Silenced`](super::Silenced) tag itself is a weapon-side sibling component (fitted by
//! the GTW-549 [`Silence`](super::AttachmentEffect::Silence) attachment effect). This module
//! owns only the shared RESOLUTION both producers call — relocated here (GTW-549) out of the
//! ripped-out GTW-542 `attachment` module so the gate survives the attachment-model rework
//! unchanged.

use bevy::prelude::{Entity, Query, With};

use super::Silenced;
use crate::fire::{MeleeQuery, WieldsQuery};

/// Whether a `shooter`'s **RANGED** weapon carries the [`Silenced`] tag — the shared gate
/// BOTH loud-signal producers ([`apply_suppression`](crate::suppression::apply_suppression)
/// and [`reaction_trigger`](crate::reaction::reaction_trigger)) key off so a silenced shot
/// propagates neither SUPPRESSION nor REACTION/REVEAL.
///
/// Resolves `shooter → Wields → the RANGED weapon entity` (EXCLUDING the melee weapon via
/// the `melee` probe — the `dispatch_fire` resolution, so the gun's tag is read, never a
/// melee weapon's) then probes that weapon entity for [`Silenced`] via `silenced`. A
/// shooter with no resolvable ranged weapon is treated as NOT silenced (`false`) — the
/// producers then fall through to their normal behaviour rather than silently swallowing
/// a shot from an unresolved source. Faction-blind (both factions' silenced shots stay
/// quiet).
///
/// The `silenced` probe is a `Query<(), With<Silenced>>` — a marker-only archetype filter
/// (the cheapest presence probe), disjoint from the wield/melee queries. Framework
/// system-param plumbing throughout (the no-bare-types carve-out).
#[must_use]
pub fn shooter_weapon_silenced(
    shooter: Entity,
    wields: &WieldsQuery,
    melee: &MeleeQuery,
    silenced: &Query<(), With<Silenced>>,
) -> bool {
    wields
        .get(shooter)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .is_some_and(|weapon| silenced.get(weapon).is_ok())
}
