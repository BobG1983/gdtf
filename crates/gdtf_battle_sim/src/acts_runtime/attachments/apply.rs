//! [`apply_pending_attachments`] — apply each spawned weapon's resolved attachment effects
//! to the weapon entity, once, via the commands extension.

use bevy::prelude::{Commands, Entity, Query};

use crate::weapon::{AttachToWeaponExt, PendingAttachments};

/// Apply the resolved [`AttachmentEffect`](crate::weapon::AttachmentEffect)s a weapon carries
/// on its [`PendingAttachments`] marker to the weapon entity, then REMOVE the marker so the
/// apply is one-shot (GTW-549).
///
/// For each weapon entity carrying a [`PendingAttachments`] component, iterates its resolved
/// effects and invokes each via the
/// [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon) commands extension
/// (the mandated post-spawn [`EntityCommand`](bevy::ecs::system::EntityCommand) — the weapon
/// entity exists now, its stat components having materialized from the scene). Each effect's
/// isolated `apply_to_weapon` mutates the correct component (`Aim` → `Accuracy`, `ExtraAmmo` →
/// `Magazine`, `Silence` → `Silenced`, `Stability` → `WeaponBraceBonus`, …). After queuing the
/// effect commands the [`PendingAttachments`] marker is removed, so a weapon is applied
/// EXACTLY ONCE (a subsequent frame's query no longer matches it — no double-apply of an
/// additive effect).
///
/// A weapon with an EMPTY pending list (a weapon with no attachments) still matches the query
/// on the frame its scene materialized; the loop applies nothing and the marker is removed —
/// a no-op, so an un-attached weapon is byte-identical. Fail-safe: the effect closures NO-OP
/// on an absent target component (a mis-seeded weapon), never panic.
///
/// `bevy-traps.md` #7 — query + [`Commands`] only, no `&mut World`. The removal is queued on
/// [`Commands`], so it takes effect at the next command flush; the query's `&PendingAttachments`
/// borrow is read-only (the effects mutate through the deferred `attach_to_weapon` closures,
/// not through this query), so there is no query/command conflict.
pub fn apply_pending_attachments(
    pending: Query<(Entity, &PendingAttachments)>,
    mut commands: Commands,
) {
    for (weapon, attachments) in &pending {
        for effect in attachments.effects() {
            // Clone the effect into the deferred command (the closure is `'static`). Each
            // AttachmentEffect enum value delegates to its isolated behaviour type.
            commands.attach_to_weapon(weapon, effect.clone());
        }
        // One-shot: drop the marker so a later frame never re-applies these additive effects.
        commands.entity(weapon).remove::<PendingAttachments>();
    }
}
