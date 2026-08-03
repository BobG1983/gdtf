use bevy::prelude::EntityWorldMut;
use serde::{Deserialize, Serialize};

use super::{
    AimDelta, ApplyAim, ApplyAttachmentEffect, ApplyBrace, ApplyDamage, ApplyDamageTypeOverride,
    ApplyExtraAmmo, ApplyFatalBias, ApplyGainFireMode, ApplyPenetration, ApplyReloadTime,
    ApplyShove, ApplyShred, ApplySilence, ApplyStability, ReloadTimeScale, WeaponBraceBonus,
};
use crate::weapon::{
    DamageType, FatalBias, FireModeSpec, MagazineSize, WeaponDamage, WeaponPunch, WeaponShred,
};

/// per-item magnitude. `#[derive(Deserialize)]` so the list round-trips from RON by
/// the on-disk form is this closed enum); `#[derive(Serialize)]` so the content editor's
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttachmentEffect {
                        Aim(AimDelta),
                        Stability(WeaponBraceBonus),
                    GainFireMode(FireModeSpec),
                ExtraAmmo(MagazineSize),
                            ReloadTime(ReloadTimeScale),
                    Silence,
                Penetration(WeaponPunch),
            DamageTypeOverride(DamageType),
                /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
        Damage(WeaponDamage),
                /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
        Shred(WeaponShred),
                    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing weapon
        FatalBias(FatalBias),
                        /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing `Stable` tag.
    Brace,
                    /// USER-REVIEW extra (defensible default; ship + log). Reuses the existing `Shove` tag.
    Shove,
}

impl ApplyAttachmentEffect for AttachmentEffect {
                                    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        match self {
            Self::Aim(delta) => ApplyAim::new(*delta).apply_to_weapon(weapon),
            Self::Stability(bonus) => ApplyStability::new(*bonus).apply_to_weapon(weapon),
            Self::GainFireMode(mode) => ApplyGainFireMode::new(*mode).apply_to_weapon(weapon),
            Self::ExtraAmmo(size) => ApplyExtraAmmo::new(*size).apply_to_weapon(weapon),
            Self::ReloadTime(scale) => ApplyReloadTime::new(*scale).apply_to_weapon(weapon),
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
