//! The shared **silenced-weapon gate** — [`shooter_weapon_silenced`], the predicate BOTH
//! loud-signal producers key off so a silenced shot propagates neither SUPPRESSION nor
//! REACTION/REVEAL.
//!
//! The [`Silenced`](super::Silenced) tag itself is a weapon-side sibling component (fitted by
//! the GTW-549 [`Silence`](crate::effects::attachments::AttachmentEffect::Silence) attachment effect). This module
//! owns only the shared RESOLUTION both producers call — relocated here (GTW-549) out of the
//! ripped-out GTW-542 `attachment` module so the gate survives the attachment-model rework
//! unchanged.

use bevy::prelude::{Entity, Query, With};

use super::Silenced;
use crate::fire::{MeleeQuery, MountedQuery, WieldsQuery};

/// Whether the weapon a `shooter` would **FIRE** carries the [`Silenced`] tag — the shared
/// gate BOTH loud-signal producers
/// ([`apply_suppression`](crate::suppression::apply_suppression) and
/// [`reaction_trigger`](crate::reaction::reaction_trigger)) key off so a silenced shot
/// propagates neither SUPPRESSION nor REACTION/REVEAL.
///
/// Resolves `shooter → Wields → the FIRING weapon entity` through the ONE shared fire-weapon
/// preference rule ([`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon), GTW-660):
/// PREFER the mounted weapon (via the `mounted` probe — the emplacement's bolted-down gun a
/// manning ganger actually fires, GTW-543), else the carried ranged weapon (EXCLUDING the
/// melee weapon via the `melee` probe, GTW-505). This is the SAME weapon `dispatch_fire`
/// dispatches, so a MOUNTED shooter's silenced read tracks the mount it fires — never the
/// carried gun it isn't (GTW-674 — a silenced carried pistol no longer wrongly silences
/// emplacement fire, and a silenced mount is correctly quiet). It then probes that weapon
/// entity for [`Silenced`] via `silenced`. A shooter with no resolvable firing weapon is
/// treated as NOT silenced (`false`) — the producers then fall through to their normal
/// behaviour rather than silently swallowing a shot from an unresolved source. Faction-blind
/// (both factions' silenced shots stay quiet).
///
/// The `mounted` / `melee` / `silenced` probes are marker-only archetype filters (the
/// cheapest presence probes), disjoint from the wield query and each other. Framework
/// system-param plumbing throughout (the no-bare-types carve-out).
#[must_use]
pub fn shooter_weapon_silenced(
    shooter: Entity,
    wields: &WieldsQuery,
    mounted: &MountedQuery,
    melee: &MeleeQuery,
    silenced: &Query<(), With<Silenced>>,
) -> bool {
    wields
        .get(shooter)
        .ok()
        .and_then(|w| {
            w.firing_weapon(
                |entity| mounted.get(entity).is_ok(),
                |entity| melee.get(entity).is_ok(),
            )
        })
        .is_some_and(|weapon| silenced.get(weapon).is_ok())
}
