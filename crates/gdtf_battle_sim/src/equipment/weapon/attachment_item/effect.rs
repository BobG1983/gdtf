//! The closed **attachment-effect vocabulary** — the [`AttachmentEffect`] a
//! [`AttachmentSpec`](super::AttachmentSpec) lists, one entry per fitted effect, each
//! carrying its OWN magnitude/payload (GTW-549 — the data-driven rework that SUPERSEDES the
//! GTW-542 closed `AttachTag` enum + global-tuning model, removed).
//!
//! ## The corrected design (GTW-549)
//!
//! GTW-542 modelled attachments as a hardcoded closed `AttachTag` enum whose effects were
//! baked into folder-functions, with the effect MAGNITUDES living in GLOBAL tuning
//! (`combat.tuning.ron`). GTW-549 fixes three defects:
//!
//! 1. A SIGHT now boosts AIM (the [`Accuracy`](crate::weapon::Accuracy) stat — the in-cone
//!    concentration lever), NOT stability — [`Aim`](AttachmentEffect::Aim). Distinct from
//!    the graduated [`Stability`](AttachmentEffect::Stability) brace contribution.
//! 2. Every magnitude lives ON the effect (its named-newtype payload), NOT in global tuning.
//! 3. Attachments are DATA-DRIVEN RON items (a [`AttachmentSpec`](super::AttachmentSpec)
//!    holding a `Vec<AttachmentEffect>`), NOT a closed code enum baked into folder-functions.
//!
//! ## The serde name↔type bridge + effect isolation
//!
//! This vocabulary is a closed, serde-deserializable enum (RON cannot deserialize trait
//! objects, so the on-disk shape is this closed enum keyed by variant name). Each variant's
//! BEHAVIOUR is a CONCEPTUALLY-ISOLATED type in the private `apply` module impl-ing the
//! [`ApplyAttachmentEffect`] trait (its method IS its behaviour — no central logic `match`, no
//! folder-fn), and this enum's own [`ApplyAttachmentEffect`] impl is a THIN delegation `match`
//! that constructs the isolated type and calls it. Adding a new effect means adding ONE
//! variant here, ONE isolated type, ONE `impl`, and ONE mechanical delegation arm — all in
//! this module. The [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands
//! extension invokes the trait as a post-spawn
//! [`EntityCommand`](bevy::ecs::system::EntityCommand) against the (already-spawned) weapon
//! entity.
//!
//! Every magnitude-carrying variant holds its magnitude as a NAMED-newtype payload (the
//! no-bare-types rule), reusing an existing weapon newtype where one fits or a dedicated
//! per-item magnitude newtype otherwise. The no-payload variants
//! ([`Silence`](AttachmentEffect::Silence) / [`Brace`](AttachmentEffect::Brace) /
//! [`Shove`](AttachmentEffect::Shove)) map to boolean weapon tags.

use bevy::prelude::EntityWorldMut;
use serde::Deserialize;

use super::{
    apply::{
        ApplyAim, ApplyAttachmentEffect, ApplyBrace, ApplyDamage, ApplyDamageTypeOverride,
        ApplyExtraAmmo, ApplyFastReload, ApplyFatalBias, ApplyGainFireMode, ApplyPenetration,
        ApplyShove, ApplyShred, ApplySilence, ApplyStability,
    },
    magnitude::{AimDelta, ReloadScale, WeaponBraceBonus},
};
use crate::weapon::{
    DamageType, FatalBias, FireModeSpec, MagazineSize, WeaponDamage, WeaponPunch, WeaponShred,
};

/// One **effect** a data-driven attachment item applies to its weapon — the closed,
/// serde-deserializable vocabulary an [`AttachmentSpec`](super::AttachmentSpec) lists
/// (GTW-549, PHASE 1).
///
/// A weapon references an attachment item BY KEY in its
/// [`attachments`](crate::weapon::WeaponSpec::attachments); the resolved item
/// holds a `Vec<AttachmentEffect>`, each variant naming a distinct stat lever with its
/// per-item magnitude. `#[derive(Deserialize)]` so the list round-trips from RON by
/// variant name (the serde name↔type bridge — RON cannot deserialize trait objects, so
/// the on-disk form is this closed enum). NOT a `Component` — it is authoring DATA.
///
/// It impls [`ApplyAttachmentEffect`] (PHASE 2) by DELEGATING each variant to its
/// isolated behaviour type in the private `apply` module; the
/// [`attach_to_weapon`](super::AttachToWeaponExt::attach_to_weapon) commands extension
/// applies it to the weapon entity post-spawn.
///
/// Each magnitude-carrying variant's payload is a NAMED newtype (no-bare-types), and the
/// magnitude lives HERE (on the item), never in global tuning — the headline GTW-549 fix.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AttachmentEffect {
    /// **Aim** — a precision optic that raises the weapon's
    /// [`Accuracy`](crate::weapon::Accuracy) (the §1b in-cone concentration exponent) by
    /// the per-item [`AimDelta`]. THIS IS THE HEADLINE GTW-549 FIX: a sight boosts AIM
    /// (clustering the in-cone draw toward centre), NOT stability (which would narrow the
    /// cone) — the two are distinct combat levers (`docs/combat/resolution.md` §1b).
    Aim(AimDelta),
    /// **Stability** — a bipod / brace that adds a graduated per-item
    /// [`WeaponBraceBonus`] of §1a stability-score points (the brace seam — an additive
    /// stability contribution, the [`SuppressionStability`](crate::SuppressionStability) /
    /// emplacement precedent). GRADUATED, not the boolean [`Brace`](AttachmentEffect::Brace)
    /// tag — a per-item magnitude.
    Stability(WeaponBraceBonus),
    /// **`GainFireMode`** — a conversion kit that ADDS one
    /// [`FireModeSpec`](crate::weapon::FireModeSpec) to the weapon's
    /// [`FireMode`](crate::weapon::FireMode) selector (a new firing mode with its own cone
    /// mult / TU% / shots).
    GainFireMode(FireModeSpec),
    /// **`ExtraAmmo`** — an oversized magazine that ADDS its
    /// [`MagazineSize`](crate::weapon::MagazineSize) capacity to the weapon's magazine
    /// (more rounds before a reload).
    ExtraAmmo(MagazineSize),
    /// **`FastReload`** — a speed-loader that SCALES the weapon's
    /// [`ReloadTu`](crate::magazine::ReloadTu) reload cost by its per-item
    /// [`ReloadScale`] (`< 1.0` → a faster reload). GTW-549 re-homes the magnitude ONTO
    /// the item (the GTW-542 model read a GLOBAL tuning factor — the exact defect this
    /// rework fixes).
    FastReload(ReloadScale),
    /// **Silence** — a suppressor (no payload): fits the
    /// [`Silenced`](crate::weapon::Silenced)`(true)` tag so the weapon's shots propagate
    /// neither SUPPRESSION nor REACTION/REVEAL (both producer gates read the tag). PRESERVES
    /// the GTW-542 `Silenced` component + BOTH its producer gates.
    Silence,
    /// **Penetration** — cursed / armour-piercing rounds that ADD their
    /// [`WeaponPunch`](crate::weapon::WeaponPunch) to the weapon's penetration (ignores
    /// more armour).
    Penetration(WeaponPunch),
    /// **`DamageTypeOverride`** — a toxic / elemental coating that OVERRIDES the weapon's
    /// emitted [`DamageType`](crate::weapon::DamageType) (a matchup-wheel re-key).
    DamageTypeOverride(DamageType),
    /// **Damage** — a brutal counterweight / hotter load that ADDS its
    /// [`WeaponDamage`](crate::weapon::WeaponDamage) to the weapon's base damage.
    ///
    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
    /// newtype.
    Damage(WeaponDamage),
    /// **Shred** — a serrated attachment that ADDS its
    /// [`WeaponShred`](crate::weapon::WeaponShred) to the weapon's armour-durability damage.
    ///
    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
    /// newtype.
    Shred(WeaponShred),
    /// **`FatalBias`** — a savage muzzle that ADDS its
    /// [`FatalBias`](crate::weapon::FatalBias) to the weapon's severity-score addend
    /// (nastier §6 wound buckets).
    ///
    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
    /// newtype.
    FatalBias(FatalBias),
    /// **Brace** — an unconditional rigid brace (no payload): fits the
    /// [`Stable`](crate::weapon::Stable)`(true)` tag (the §1a unconditional brace). DISTINCT
    /// from the GRADUATED [`Stability`](AttachmentEffect::Stability) — a boolean tag, not a
    /// magnitude.
    ///
    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing `Stable` tag.
    Brace,
    /// **Shove** — a knockback attachment (no payload): fits the
    /// [`Shove`](crate::weapon::Shove)`(true)` tag (GTW-525 — knocks the target back one
    /// cell on a connecting strike).
    ///
    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing `Shove` tag.
    Shove,
}

impl ApplyAttachmentEffect for AttachmentEffect {
    /// Apply this effect to its `weapon` entity by DELEGATING to the isolated behaviour type
    /// in the private `apply` module (GTW-549 PHASE 2 — the effect-isolation architecture).
    ///
    /// This `match` carries NO logic — every arm is a one-line mechanical delegation that
    /// constructs the variant's isolated [`ApplyAttachmentEffect`] type from its payload and
    /// forwards the call. The BEHAVIOUR (which component, additive vs override, the rebuild)
    /// lives entirely in that isolated type, so adding an effect touches ONE variant + ONE
    /// isolated type + this ONE delegation line — never a central logic branch.
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        match self {
            Self::Aim(delta) => ApplyAim::new(*delta).apply_to_weapon(weapon),
            Self::Stability(bonus) => ApplyStability::new(*bonus).apply_to_weapon(weapon),
            Self::GainFireMode(mode) => ApplyGainFireMode::new(*mode).apply_to_weapon(weapon),
            Self::ExtraAmmo(size) => ApplyExtraAmmo::new(*size).apply_to_weapon(weapon),
            Self::FastReload(scale) => ApplyFastReload::new(*scale).apply_to_weapon(weapon),
            Self::Silence => ApplySilence.apply_to_weapon(weapon),
            Self::Penetration(punch) => ApplyPenetration::new(*punch).apply_to_weapon(weapon),
            Self::DamageTypeOverride(ty) => {
                ApplyDamageTypeOverride::new(*ty).apply_to_weapon(weapon);
            }
            Self::Damage(dmg) => ApplyDamage::new(*dmg).apply_to_weapon(weapon),
            Self::Shred(shred) => ApplyShred::new(*shred).apply_to_weapon(weapon),
            Self::FatalBias(bias) => ApplyFatalBias::new(*bias).apply_to_weapon(weapon),
            Self::Brace => ApplyBrace.apply_to_weapon(weapon),
            Self::Shove => ApplyShove.apply_to_weapon(weapon),
        }
    }
}
