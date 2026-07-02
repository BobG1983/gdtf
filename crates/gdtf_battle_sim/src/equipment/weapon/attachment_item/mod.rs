//! Data-driven **weapon attachments** (GTW-549 — child of GTW-551 → GTW-17) —
//! the RON-authored attachment ITEM model that SUPERSEDES the GTW-542 closed `AttachTag`
//! enum + folder-function + global-tuning model (all removed).
//!
//! ## The corrected design
//!
//! GTW-542 modelled attachments as a hardcoded closed `AttachTag` enum whose effects were
//! baked into folder-functions, with the effect MAGNITUDES living in GLOBAL tuning
//! (`combat.tuning.ron`) and a SIGHT wrongly modelled as a stability (recoil) bonus. GTW-549
//! reworks them as DATA-DRIVEN RON ITEMS (folder-loaded + hot-reloadable, exactly like
//! weapons / armor / melee): each [`AttachmentSpec`] carries a display name + a typed list of
//! [`AttachmentEffect`]s, and EACH effect carries its OWN magnitude (never in global tuning).
//! Weapons reference attachment items BY KEY ([`AttachmentName`]) — authored once, referenced
//! by many — and a SIGHT now boosts AIM (the [`Accuracy`](crate::weapon::Accuracy) stat), not
//! stability (the [`Aim`](AttachmentEffect::Aim) effect, the headline fix).
//!
//! ## The effect-isolation architecture (user ruling, 2026-07-02)
//!
//! Each effect's BEHAVIOUR is a CONCEPTUALLY-ISOLATED type ([`apply`]) impl-ing the
//! [`ApplyAttachmentEffect`] trait whose one method IS the effect, invoked via the
//! [`AttachToWeaponExt`] commands extension
//! ([`attach_to_weapon`](AttachToWeaponExt::attach_to_weapon), [`commands`]) as a deferred
//! [`EntityCommand`](bevy::ecs::system::EntityCommand) against the (post-spawn) weapon entity.
//! The closed serde [`AttachmentEffect`] enum ([`effect`]) is the RON name↔type bridge (RON
//! cannot deserialize trait objects); it impls the trait by THIN mechanical delegation to each
//! isolated type. Adding a new effect = ONE serde variant + ONE isolated type + ONE `impl` +
//! ONE delegation arm — NO central logic `match`, NO folder-fn, NO authoring step across the
//! codebase. At battle setup a weapon's authored
//! [`attachments`](crate::weapon::WeaponSpec::attachments) keys resolve against the
//! [`AttachmentRegistry`] into a [`PendingAttachments`](crate::weapon::PendingAttachments)
//! marker on the spawned weapon, which the post-spawn
//! [`apply_pending_attachments`](crate::acts_runtime::attachments::apply_pending_attachments) system applies
//! via the extension (the deferred-spawn bridge — the weapon entity's stat components exist
//! once its scene materializes).
//!
//! Code-health: this concern is a dir-module split by responsibility mirroring the
//! melee layout — the effect vocabulary ([`effect`]), the isolated effect behaviours +
//! trait ([`apply`]), the commands extension ([`commands`]), the per-item magnitude
//! newtypes ([`magnitude`]), the item key ([`key`]), the authoring spec ([`spec`]), and the
//! registry ([`registry`]). This `mod.rs` is wiring-only; every public path is preserved via
//! the re-exports below.

mod apply;
mod commands;
mod effect;
mod key;
mod magnitude;
mod registry;
mod spec;

#[cfg(test)]
mod apply_test;
#[cfg(test)]
mod test;

pub use apply::{
    ApplyAim, ApplyAttachmentEffect, ApplyBrace, ApplyDamage, ApplyDamageTypeOverride,
    ApplyExtraAmmo, ApplyFastReload, ApplyFatalBias, ApplyGainFireMode, ApplyPenetration,
    ApplyShove, ApplyShred, ApplySilence, ApplyStability,
};
pub use commands::AttachToWeaponExt;
pub use effect::AttachmentEffect;
pub use key::AttachmentName;
pub use magnitude::{AimDelta, ReloadScale, WeaponBraceBonus};
pub use registry::AttachmentRegistry;
pub use spec::AttachmentSpec;
