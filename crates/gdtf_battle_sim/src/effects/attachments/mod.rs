//! The **weapon-attachment effect palette** (GTW-558 — child of GTW-551 → GTW-17; the
//! one-file-per-effect re-home of the GTW-549 effect-isolation architecture).
//!
//! ## The shape
//!
//! - `apply_effect` — the shared [`ApplyAttachmentEffect`](crate::effects::attachments::ApplyAttachmentEffect)
//!   trait (the palette contract: one method IS an effect's behaviour).
//! - `effect` — the closed serde [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)
//!   vocabulary (the RON name↔type bridge) + its ONE thin 13-arm delegation
//!   `impl ApplyAttachmentEffect` (each arm constructs the per-effect `ApplyX` and
//!   forwards — NO logic).
//! - ONE SELF-CONTAINED FILE PER EFFECT (`aim`, `stability`, `reload_time`,
//!   `gain_fire_mode`, `extra_ammo`, `silence`, `penetration`, `damage_type_override`,
//!   `damage`, `shred`, `fatal_bias`, `brace`, `shove`), each holding its per-item magnitude
//!   newtype (where it has one) + its isolated `ApplyX` struct + its
//!   `impl ApplyAttachmentEffect` + a `#[cfg(test)]` unit test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new effect = ONE new per-effect file + ONE
//! [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect) variant + ONE
//! delegation arm (in `effect`) + ONE `mod` line here. No central logic `match`, no
//! folder-fn, no authoring step scattered across the codebase. The mechanics that RESOLVE +
//! APPLY the palette (the registry / commands extension / spawn-applier) live under
//! [`equipment::attachments`](crate::equipment::attachments) and invoke the trait
//! generically — they NEVER match on the effect enum.
//!
//! Dependency direction: this palette depends on [`crate::weapon`] (the stat newtypes an
//! effect targets); the mechanics depend on this palette. Acyclic.

mod aim;
mod apply_effect;
mod brace;
mod damage;
mod damage_type_override;
mod effect;
mod extra_ammo;
mod fatal_bias;
mod gain_fire_mode;
mod penetration;
mod reload_time;
mod shove;
mod shred;
mod silence;
mod stability;

#[cfg(test)]
mod tests;

pub use aim::{AimDelta, ApplyAim};
pub use apply_effect::ApplyAttachmentEffect;
pub use brace::ApplyBrace;
pub use damage::ApplyDamage;
pub use damage_type_override::ApplyDamageTypeOverride;
pub use effect::AttachmentEffect;
pub use extra_ammo::ApplyExtraAmmo;
pub use fatal_bias::ApplyFatalBias;
pub use gain_fire_mode::ApplyGainFireMode;
pub use penetration::ApplyPenetration;
pub use reload_time::{ApplyReloadTime, ReloadTimeScale};
pub use shove::ApplyShove;
pub use shred::ApplyShred;
pub use silence::ApplySilence;
pub use stability::{ApplyStability, WeaponBraceBonus};
