//! The **`commands.attach_to_weapon` extension** (GTW-549, PHASE 2 — child of GTW-551 →
//! GTW-17): the [`AttachToWeaponExt`] trait that adds an
//! [`attach_to_weapon`](AttachToWeaponExt::attach_to_weapon) method to Bevy's
//! [`Commands`], queuing an attachment effect as a deferred
//! [`EntityCommand`](bevy::ecs::system::EntityCommand) against a (post-spawn) weapon entity.
//!
//! ## Why a commands extension (the user ruling, 2026-07-02)
//!
//! An [`AttachmentEffect`](super::AttachmentEffect) is applied to a weapon that already
//! exists (spawned by the BSN wielded-weapon scene). The natural seam is therefore a
//! DEFERRED command against that entity — `commands.attach_to_weapon(weapon, effect)` —
//! which Bevy runs at the next command flush with exclusive access to the entity via an
//! [`EntityWorldMut`](bevy::prelude::EntityWorldMut). The effect's
//! [`apply_to_weapon`](super::ApplyAttachmentEffect::apply_to_weapon) IS the command body;
//! the extension is a thin, ergonomic wrapper so a caller never writes the closure or the
//! `entity(..).queue(..)` plumbing by hand.

use bevy::prelude::{Commands, Entity, EntityWorldMut};

use super::ApplyAttachmentEffect;

/// The **`commands.attach_to_weapon`** extension (GTW-549 PHASE 2) — adds
/// [`attach_to_weapon`](Self::attach_to_weapon) to Bevy's [`Commands`] so a caller can fit
/// an attachment effect onto an already-spawned weapon entity through the ordinary command
/// buffer.
///
/// The single entry point the effect-isolation architecture exposes: the setup path
/// resolves a weapon's attachment-item keys against the
/// [`AttachmentRegistry`](super::AttachmentRegistry), then calls this once per effect. It is
/// a trait on [`Commands`] (the ext-trait idiom) rather than a free function so the call
/// reads as `commands.attach_to_weapon(..)` at the seam.
pub trait AttachToWeaponExt {
    /// Queue `effect` to be applied to the `weapon` entity at the next command flush.
    ///
    /// The `effect` is moved into a deferred
    /// [`EntityCommand`](bevy::ecs::system::EntityCommand) closure that Bevy runs with
    /// exclusive access to the `weapon` entity (an
    /// [`EntityWorldMut`](bevy::prelude::EntityWorldMut) — the ticket's sanctioned
    /// carve-out); the closure just forwards to the effect's
    /// [`apply_to_weapon`](super::ApplyAttachmentEffect::apply_to_weapon). Because it is
    /// DEFERRED, the caller may spawn the weapon and attach its effects in the same system —
    /// the effects land after the spawn is flushed. Returns `&mut Self` so calls chain
    /// (one per effect in the resolved item).
    fn attach_to_weapon(
        &mut self,
        weapon: Entity,
        effect: impl ApplyAttachmentEffect + Send + 'static,
    ) -> &mut Self;
}

impl AttachToWeaponExt for Commands<'_, '_> {
    fn attach_to_weapon(
        &mut self,
        weapon: Entity,
        effect: impl ApplyAttachmentEffect + Send + 'static,
    ) -> &mut Self {
        // The closure `FnOnce(EntityWorldMut)` IS the EntityCommand (the Bevy 0.19 blanket
        // impl); it forwards to the isolated effect's behaviour. `queue` (not
        // `queue_silenced`) so a genuine command error surfaces through the default handler.
        self.entity(weapon)
            .queue(move |mut entity: EntityWorldMut| {
                effect.apply_to_weapon(&mut entity);
            });
        self
    }
}
