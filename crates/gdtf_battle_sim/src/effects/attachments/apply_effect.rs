//! The **shared apply contract** for the attachment-effect palette — the
//! [`ApplyAttachmentEffect`] trait whose one method IS an effect's behaviour (GTW-558,
//! extracted from the GTW-549 effect-isolation architecture).
//!
//! Every isolated per-effect `ApplyX` type (one per file in this module) impls this trait;
//! the closed [`AttachmentEffect`](super::AttachmentEffect) serde enum impls it too, by THIN
//! mechanical delegation to each variant's isolated type. The mechanics never match on the
//! enum — they invoke this trait generically (the
//! [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon)
//! commands extension queues it as a deferred
//! [`EntityCommand`](bevy::ecs::system::EntityCommand) against the weapon entity).

use bevy::prelude::EntityWorldMut;

/// One attachment effect's **isolated behaviour** — the trait each concrete effect type
/// impls, its single method BEING the effect (GTW-558; the effect-isolation contract, user
/// ruling 2026-07-02).
///
/// The whole point: an effect's logic lives in exactly ONE place — its own type's
/// [`apply_to_weapon`](Self::apply_to_weapon) — not in a central `match`, a folder-fn, or a
/// scatter of authoring steps. The [`AttachmentEffect`](super::AttachmentEffect) serde enum
/// impls this by DELEGATING each variant to its isolated type, and the
/// [`attach_to_weapon`](crate::equipment::attachments::AttachToWeaponExt::attach_to_weapon)
/// commands extension invokes it as a deferred
/// [`EntityCommand`](bevy::ecs::system::EntityCommand) against the weapon entity.
pub trait ApplyAttachmentEffect {
    /// Apply this effect to the (already-spawned) `weapon` entity — its whole behaviour.
    ///
    /// Runs inside a deferred [`EntityCommand`](bevy::ecs::system::EntityCommand) with
    /// exclusive access to the ONE weapon entity (the ticket's
    /// [`EntityWorldMut`] carve-out). An implementation reads / inserts / rebuilds the
    /// weapon's own components ONLY; it never spawns or touches another entity. When the
    /// target component is absent (a mis-seeded weapon) the effect is a NO-OP rather than a
    /// panic (fail-safe, the `let … else` / `if let` guards in each effect file).
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>);
}
