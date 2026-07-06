//! The **weapon-attachment mechanics** (GTW-558 — child of GTW-551 → GTW-17) — the
//! authoring/resolution/application machinery that RESOLVES a weapon's authored attachment
//! keys and APPLIES the resolved effect palette. The palette itself (the closed
//! [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect) vocabulary + one
//! isolated `ApplyX` behaviour per effect) lives under
//! [`crate::effects::attachments`]; this module owns only the mechanics.
//!
//! ## The shape (a dir-module split by responsibility, the melee precedent)
//!
//! - `key` — the [`AttachmentName`](crate::equipment::attachments::AttachmentName) item key
//!   (a weapon references an attachment by it).
//! - `slot` — the GTW-554 slot vocabulary: the closed
//!   [`AttachmentSlot`](crate::equipment::attachments::AttachmentSlot) enum, the
//!   [`SlotCapacity`](crate::equipment::attachments::SlotCapacity) count, and the
//!   [`WeaponSlots`](crate::equipment::attachments::WeaponSlots) declaration a weapon authors.
//! - `fit` — the GTW-554 fit gate
//!   ([`attachment_fits`](crate::equipment::attachments::attachment_fits)) and the shared
//!   setup-time resolution
//!   ([`resolve_pending_attachments`](crate::equipment::attachments::resolve_pending_attachments))
//!   both weapon-spawn paths call: an item fits ONLY into a declared slot with free capacity
//!   (fit IS mechanics, so it lives here).
//! - `spec` — the [`AttachmentSpec`](crate::equipment::attachments::AttachmentSpec) authoring
//!   item (`display_name` + `slot` + `Vec<AttachmentEffect>`).
//! - `registry` — the [`AttachmentRegistry`](crate::equipment::attachments::AttachmentRegistry)
//!   key→spec map the folder loader builds and setup resolves against.
//! - `commands` — the [`AttachToWeaponExt`](crate::equipment::attachments::AttachToWeaponExt)
//!   commands extension; it applies an item GENERICALLY
//!   (`for e in effects { e.apply_to_weapon(weapon) }`) via the palette's shared trait — it
//!   NEVER matches on the effect enum.
//! - `apply` — the post-spawn
//!   [`apply_pending_attachments`](crate::equipment::attachments::apply_pending_attachments)
//!   system that runs a spawned weapon's resolved effects (the deferred-spawn bridge; a
//!   general mechanic, so it lives here with the mechanics, NOT in the act runtime
//!   (`crate::acts`)).
//!
//! ## Dependency direction (acyclic)
//!
//! `equipment::attachments` (mechanics) → [`effects::attachments`](crate::effects::attachments)
//! (palette) → [`equipment::weapon`](crate::weapon) (the stat newtypes an effect targets).
//! The mechanics never own effect BEHAVIOUR; the palette never owns mechanics.
//!
//! This `mod.rs` is wiring-only; every public path is preserved via the re-exports below.

mod apply;
mod commands;
mod fit;
mod key;
mod registry;
mod slot;
mod spec;

#[cfg(test)]
mod tests;

pub use apply::apply_pending_attachments;
pub use commands::AttachToWeaponExt;
pub use fit::{FitRejection, attachment_fits, resolve_pending_attachments};
pub use key::AttachmentName;
pub use registry::AttachmentRegistry;
pub use slot::{AttachmentSlot, SlotCapacity, WeaponSlots};
pub use spec::AttachmentSpec;
