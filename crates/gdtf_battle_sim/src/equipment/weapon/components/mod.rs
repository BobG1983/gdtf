//! The per-stat weapon **components** — each weapon NUMBER as its own
//! `#[derive(Component)]` newtype (GTW-200), the [`DamageType`] vocabulary, the
//! [`WeaponName`], and the unit [`Weapon`] marker. Every numeric leaf is a named
//! newtype (no-bare-types): a private inner value, a derived [`Deref`](bevy::prelude::Deref), and
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar.

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
