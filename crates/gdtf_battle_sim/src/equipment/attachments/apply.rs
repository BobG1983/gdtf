//! The **attachment-application mechanic** (GTW-549, child GTW-551; GTW-558 re-homed from
//! `acts_runtime::attachments` into the attachment MECHANICS module) — the
//! [`apply_pending_attachments`] system that applies a spawned weapon's resolved
//! [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)s (carried on its
//! [`PendingAttachments`](crate::weapon::PendingAttachments) marker) to the weapon entity via
//! the [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands extension.
//!
//! ## Why a system (not an inline setup call)
//!
//! The wielded-weapon scene is spawned via
//! [`queue_spawn_related_scenes`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes),
//! whose components materialize DEFERRED (on the `SpawnScene` schedule), so
//! [`setup_battle`](crate::situation::setup_battle) has no live weapon
//! [`Entity`](bevy::prelude::Entity) to `attach_to_weapon` inline — a command queued at setup
//! time would flush BEFORE the weapon's stat components exist and every `get::<Accuracy>()`
//! would miss. So setup composes the resolved effects onto the weapon as the
//! [`PendingAttachments`](crate::weapon::PendingAttachments) component (part of the scene),
//! and THIS system — running after the weapon materializes — reads each effect and invokes
//! the mandated post-spawn [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon)
//! `EntityCommand` against the now-live weapon entity, then removes the marker so the apply is
//! one-shot.
//!
//! It is a general mechanic (runs a weapon's attachments at spawn), so it belongs with the
//! attachment mechanics, NOT in `acts_runtime`. Pure ECS side effect (`bevy-traps.md` #7 —
//! query / [`Commands`](bevy::prelude::Commands), no `&mut World`); the effect closures'
//! `EntityWorldMut` access is the ticket's sanctioned carve-out (see the
//! [`crate::effects::attachments`] palette docs).

use bevy::prelude::{Commands, Entity, Query};

use super::AttachToWeaponExt;
use crate::weapon::PendingAttachments;

/// Apply the resolved
/// [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)s a weapon carries on
/// its [`PendingAttachments`] marker to the weapon entity, then REMOVE the marker so the
/// apply is one-shot (GTW-549).
///
/// For each weapon entity carrying a [`PendingAttachments`] component, iterates its resolved
/// effects and invokes each via the
/// [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands extension
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
