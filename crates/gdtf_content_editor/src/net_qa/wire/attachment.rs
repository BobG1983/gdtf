//! The Attachment form's slot and effect list on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect, ReloadTimeScale, WeaponBraceBonus},
    equipment::attachments::AttachmentSlot,
    weapon::{DamageType, FatalBias, MagazineSize, WeaponDamage, WeaponPunch, WeaponShred},
};
use serde::{Deserialize, Serialize};

use super::fire_mode::FireModeSpecNet;

/// Where an attachment mounts on a weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum AttachmentSlotNet {
    /// Muzzle device.
    Muzzle,
    /// Optic or sight.
    Sight,
    /// Rail accessory.
    Rail,
    /// Magazine well.
    Magazine,
    /// Counterweight.
    Counterweight,
    /// Pommel.
    Pommel,
}

impl AttachmentSlotNet {
    /// Mirror the sim's own slot.
    pub(in crate::net_qa) const fn from_slot(slot: AttachmentSlot) -> Self {
        match slot {
            AttachmentSlot::Muzzle => Self::Muzzle,
            AttachmentSlot::Sight => Self::Sight,
            AttachmentSlot::Rail => Self::Rail,
            AttachmentSlot::Magazine => Self::Magazine,
            AttachmentSlot::Counterweight => Self::Counterweight,
            AttachmentSlot::Pommel => Self::Pommel,
        }
    }

    /// Read a client's slot back as the sim's own.
    pub(in crate::net_qa) const fn to_slot(self) -> AttachmentSlot {
        match self {
            Self::Muzzle => AttachmentSlot::Muzzle,
            Self::Sight => AttachmentSlot::Sight,
            Self::Rail => AttachmentSlot::Rail,
            Self::Magazine => AttachmentSlot::Magazine,
            Self::Counterweight => AttachmentSlot::Counterweight,
            Self::Pommel => AttachmentSlot::Pommel,
        }
    }
}

/// The damage channel an override emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum DamageTypeNet {
    /// Shock.
    Shock,
    /// Blast.
    Blast,
    /// Chemical.
    Chem,
    /// Kinetic.
    Kinetic,
    /// Plasma.
    Plasma,
    /// Rend.
    Rend,
    /// Laser.
    Las,
}

impl DamageTypeNet {
    /// Mirror the sim's own damage type.
    pub(in crate::net_qa) const fn from_damage_type(damage_type: DamageType) -> Self {
        match damage_type {
            DamageType::Shock => Self::Shock,
            DamageType::Blast => Self::Blast,
            DamageType::Chem => Self::Chem,
            DamageType::Kinetic => Self::Kinetic,
            DamageType::Plasma => Self::Plasma,
            DamageType::Rend => Self::Rend,
            DamageType::Las => Self::Las,
        }
    }

    /// Read a client's damage type back as the sim's own.
    pub(in crate::net_qa) const fn to_damage_type(self) -> DamageType {
        match self {
            Self::Shock => DamageType::Shock,
            Self::Blast => DamageType::Blast,
            Self::Chem => DamageType::Chem,
            Self::Kinetic => DamageType::Kinetic,
            Self::Plasma => DamageType::Plasma,
            Self::Rend => DamageType::Rend,
            Self::Las => DamageType::Las,
        }
    }
}

/// The accuracy an aim effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct AimDeltaNet(f32);

impl AimDeltaNet {
    /// Wrap an accuracy delta.
    pub(in crate::net_qa) const fn new(delta: f32) -> Self {
        Self(delta)
    }
}

/// The brace points a stability effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct BraceBonusNet(f32);

impl BraceBonusNet {
    /// Wrap a brace bonus.
    pub(in crate::net_qa) const fn new(bonus: f32) -> Self {
        Self(bonus)
    }
}

/// The rounds an extra-ammo effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct MagazineSizeNet(u16);

impl MagazineSizeNet {
    /// Wrap a magazine capacity.
    pub(in crate::net_qa) const fn new(rounds: u16) -> Self {
        Self(rounds)
    }
}

/// The factor a reload-time effect scales by.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ReloadTimeScaleNet(f32);

impl ReloadTimeScaleNet {
    /// Wrap a reload-time scale.
    pub(in crate::net_qa) const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// The punch a penetration effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct WeaponPunchNet(i32);

impl WeaponPunchNet {
    /// Wrap a punch value.
    pub(in crate::net_qa) const fn new(punch: i32) -> Self {
        Self(punch)
    }
}

/// The damage a damage effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct WeaponDamageNet(i32);

impl WeaponDamageNet {
    /// Wrap a damage value.
    pub(in crate::net_qa) const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

/// The shred a shred effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct WeaponShredNet(i32);

impl WeaponShredNet {
    /// Wrap a shred value.
    pub(in crate::net_qa) const fn new(shred: i32) -> Self {
        Self(shred)
    }
}

/// The bias a fatal-bias effect adds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct FatalBiasNet(f32);

impl FatalBiasNet {
    /// Wrap a fatal-bias value.
    pub(in crate::net_qa) const fn new(bias: f32) -> Self {
        Self(bias)
    }
}

/// One effect an attachment applies, with the payload its own row edits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum AttachmentEffectNet {
    /// Raise accuracy.
    Aim(AimDeltaNet),
    /// Raise the brace bonus.
    Stability(BraceBonusNet),
    /// Add a fire mode.
    GainFireMode(FireModeSpecNet),
    /// Extra magazine capacity.
    ExtraAmmo(MagazineSizeNet),
    /// Scale reload time.
    ReloadTime(ReloadTimeScaleNet),
    /// Silence the weapon.
    Silence,
    /// Extra penetration.
    Penetration(WeaponPunchNet),
    /// Override the damage type.
    DamageTypeOverride(DamageTypeNet),
    /// Extra damage.
    Damage(WeaponDamageNet),
    /// Extra shred.
    Shred(WeaponShredNet),
    /// Extra fatal bias.
    FatalBias(FatalBiasNet),
    /// Mark the weapon braceable.
    Brace,
    /// Mark the weapon shove-capable.
    Shove,
}

impl AttachmentEffectNet {
    /// Mirror the sim's own effect.
    pub(in crate::net_qa) fn from_effect(effect: &AttachmentEffect) -> Self {
        match effect {
            AttachmentEffect::Aim(delta) => Self::Aim(AimDeltaNet::new(**delta)),
            AttachmentEffect::Stability(bonus) => Self::Stability(BraceBonusNet::new(**bonus)),
            AttachmentEffect::GainFireMode(spec) => {
                Self::GainFireMode(FireModeSpecNet::from_spec(*spec))
            }
            AttachmentEffect::ExtraAmmo(size) => Self::ExtraAmmo(MagazineSizeNet::new(**size)),
            AttachmentEffect::ReloadTime(scale) => {
                Self::ReloadTime(ReloadTimeScaleNet::new(**scale))
            }
            AttachmentEffect::Silence => Self::Silence,
            AttachmentEffect::Penetration(punch) => Self::Penetration(WeaponPunchNet::new(**punch)),
            AttachmentEffect::DamageTypeOverride(damage_type) => {
                Self::DamageTypeOverride(DamageTypeNet::from_damage_type(*damage_type))
            }
            AttachmentEffect::Damage(damage) => Self::Damage(WeaponDamageNet::new(**damage)),
            AttachmentEffect::Shred(shred) => Self::Shred(WeaponShredNet::new(**shred)),
            AttachmentEffect::FatalBias(bias) => Self::FatalBias(FatalBiasNet::new(**bias)),
            AttachmentEffect::Brace => Self::Brace,
            AttachmentEffect::Shove => Self::Shove,
        }
    }

    /// Read a client's effect back as the sim's own.
    pub(in crate::net_qa) const fn to_effect(self) -> AttachmentEffect {
        match self {
            Self::Aim(delta) => AttachmentEffect::Aim(AimDelta::new(delta.0)),
            Self::Stability(bonus) => AttachmentEffect::Stability(WeaponBraceBonus::new(bonus.0)),
            Self::GainFireMode(spec) => AttachmentEffect::GainFireMode(spec.to_spec()),
            Self::ExtraAmmo(size) => AttachmentEffect::ExtraAmmo(MagazineSize::new(size.0)),
            Self::ReloadTime(scale) => AttachmentEffect::ReloadTime(ReloadTimeScale::new(scale.0)),
            Self::Silence => AttachmentEffect::Silence,
            Self::Penetration(punch) => AttachmentEffect::Penetration(WeaponPunch::new(punch.0)),
            Self::DamageTypeOverride(damage_type) => {
                AttachmentEffect::DamageTypeOverride(damage_type.to_damage_type())
            }
            Self::Damage(damage) => AttachmentEffect::Damage(WeaponDamage::new(damage.0)),
            Self::Shred(shred) => AttachmentEffect::Shred(WeaponShred::new(shred.0)),
            Self::FatalBias(bias) => AttachmentEffect::FatalBias(FatalBias::new(bias.0)),
            Self::Brace => AttachmentEffect::Brace,
            Self::Shove => AttachmentEffect::Shove,
        }
    }
}
