//! Closed set of attachment effects.

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

/// One effect an attachment can apply to a weapon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttachmentEffect {
    /// Raise accuracy.
    Aim(AimDelta),
    /// Raise stability / brace bonus.
    Stability(WeaponBraceBonus),
    /// Add a fire mode.
    GainFireMode(FireModeSpec),
    /// Extra magazine capacity.
    ExtraAmmo(MagazineSize),
    /// Scale reload time.
    ReloadTime(ReloadTimeScale),
    /// Silence the weapon.
    Silence,
    /// Extra penetration (punch).
    Penetration(WeaponPunch),
    /// Override damage type.
    DamageTypeOverride(DamageType),
    /// Extra damage.
    Damage(WeaponDamage),
    /// Extra shred.
    Shred(WeaponShred),
    /// Extra fatal bias.
    FatalBias(FatalBias),
    /// Mark the weapon as stable (braceable).
    Brace,
    /// Mark the weapon as shove-capable.
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
