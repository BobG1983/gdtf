//! Weapon component newtypes (stats, markers, tags).

mod ammo;
mod ballistics;
mod damage;
mod handling;
mod markers;
mod tags;

pub use ammo::AmmoType;
pub use ballistics::{Accuracy, BaseSpread, Kickback, Stable};
pub use damage::{DamageType, FatalBias, WeaponDamage, WeaponPunch, WeaponShred};
pub use handling::{Handedness, MagazineSize, WeaponName};
pub use markers::{MountedWeapon, Weapon};
pub use tags::{Shove, Silenced};
