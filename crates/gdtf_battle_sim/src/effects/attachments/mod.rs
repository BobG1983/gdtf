//!   `impl ApplyAttachmentEffect` + a `#[cfg(test)]` unit test.
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
