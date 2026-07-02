//! The **attachment-application runtime** (GTW-549, child GTW-551) — the
//! [`apply_pending_attachments`] system that applies a spawned weapon's resolved
//! [`AttachmentEffect`](crate::weapon::AttachmentEffect)s (carried on its
//! [`PendingAttachments`](crate::weapon::PendingAttachments) marker) to the weapon entity via
//! the [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon) commands
//! extension.
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
//! the mandated post-spawn [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon)
//! `EntityCommand` against the now-live weapon entity, then removes the marker so the apply is
//! one-shot.
//!
//! Pure ECS side effect (`bevy-traps.md` #7 — query / [`Commands`](bevy::prelude::Commands),
//! no `&mut World`); the effect closures' `EntityWorldMut` access is the ticket's sanctioned
//! carve-out (see the [`crate::weapon`] attachment-effect docs).

mod apply;

#[cfg(test)]
mod test;

pub use apply::apply_pending_attachments;
