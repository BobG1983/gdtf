//! Ranged and melee weapons: components, fire modes, specs, registry.

mod bundle;
mod components;
mod dot;
mod fire_mode;
mod firing;
mod melee;
mod registry;
mod relationship;
mod silenced;
mod spec;
mod trajectory;

#[cfg(test)]
mod test;

pub use bundle::{DamageProfile, HandlingProfile, WeaponBundle, WeaponStats};
pub use components::{
    Accuracy, AmmoType, BaseSpread, DamageType, FatalBias, Handedness, Kickback, MagazineSize,
    MountedWeapon, Shove, Silenced, Stable, Weapon, WeaponDamage, WeaponName, WeaponPunch,
    WeaponShred,
};
pub use dot::{Dot, DotDamage, DotProfile, DotTurns};
pub use fire_mode::{
    AoeRange, BlastRadius, ConeHalfAngle, FireMode, FireModeSpec, HitType, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent,
};
pub use firing::FiringWeapon;
pub use melee::{
    FISTS_KEY, FightMode, FightModeKind, FightModeSpec, MeleeDamageProfile, MeleeWeapon,
    MeleeWeaponBundle, MeleeWeaponRegistry, MeleeWeaponSpec, Reach, Strikes, TuCost,
};
pub use registry::WeaponRegistry;
pub use relationship::{WieldedBy, Wields};
pub use silenced::{ShotSilenced, shooter_weapon_silenced};
pub use spec::{PendingAttachments, WeaponSpawnSiblings, WeaponSpec};
pub use trajectory::{Lobbed, TrajectoryStyle};
