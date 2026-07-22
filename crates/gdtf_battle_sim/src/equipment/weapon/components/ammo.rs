//! The [`AmmoType`] munition vocabulary — the closed set of physical ammunition
//! classes a weapon feeds on (GTW-775, child GTW-400). The peer of the
//! [`DamageType`](super::DamageType) damage-wheel vocabulary, following the same
//! shape (a closed, serde-authored enum) but a SEPARATE concept: ammo class is
//! the physical cartridge / canister a weapon consumes, not the matchup-wheel
//! node its hits resolve on.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// The **ammunition class** a weapon feeds on — the physical munition its
/// magazine holds, and the identity a magazine's compatibility gate matches a
/// candidate against (GTW-775, child GTW-400).
///
/// This is the ammo-side vocabulary — the **peer** of the weapon/damage
/// [`DamageType`](super::DamageType) wheel, following the same pattern (a closed
/// set of serde-authored variants, a `#[derive(Component)]` newtype), but a
/// deliberately SEPARATE concept from the matchup wheel: ammo class is the
/// cartridge / flask / canister a weapon consumes, NOT the damage node its hits
/// resolve on. The two are independent — a `Grenade` munition can carry `Chem`,
/// `Blast`, or `Kinetic` damage — so a weapon names BOTH its `AmmoType`
/// ([`accepts`](crate::weapon::WeaponSpec::accepts)) and its `DamageType`.
///
/// A named domain enum, not a bare `String`/`u8` (no-bare-types): a magazine may
/// only load ammo of the class its weapon accepts — the compatibility gate
/// [`ammo_compatible`](crate::magazine::ammo_compatible). `Deserialize` /
/// `Serialize` so a weapon's authored RON names its accepted class by variant and
/// the editor round-trips it. A `#[derive(Component)]` matching the
/// [`DamageType`](super::DamageType) pattern (GTW-200) — an armed-entity-shaped value; this slice
/// stores the ACCEPTED class on [`WeaponSpec`](crate::weapon::WeaponSpec) and the
/// LOADED class on the runtime [`Magazine`](crate::magazine::Magazine).
///
/// `Default` (`AmmoType::Slug`) is a **spawn-seed sentinel only** — the runtime
/// [`Magazine::loaded_ammo`](crate::magazine::Magazine::loaded_ammo) and the
/// authored [`WeaponSpec::accepts`](crate::weapon::WeaponSpec::accepts) both
/// default to it before an authored value overwrites it, mirroring
/// [`DamageType`](super::DamageType)'s `Kinetic` default. `Slug` is chosen as the most ordinary
/// munition (the everyman slug-thrower's round); it carries no special meaning as
/// the default.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum AmmoType {
    /// Solid ballistic cartridges — cased slugs, razored darts, and bolts fed
    /// from a magazine or hopper (the stub pistol, the flechette gun, the heavy
    /// bolter, the volatile charge's crude slug-thrower).
    #[default]
    Slug,
    /// Rechargeable energy packs — the charge cells that power directed-energy
    /// weapons (the las carbine, the las-lance, the arc pistol).
    Cell,
    /// Volatile superheated-plasma flasks — the unstable canisters a plasma
    /// weapon vents (the plasma pistol, the plasma torch).
    Flask,
    /// Pressurized chemical-payload tanks — the corrosive-chem reservoir a
    /// sprayer drenches from (the chem sprayer).
    Canister,
    /// Self-contained explosive munitions — thrown grenades and launched charges
    /// (the frag / krak / gas grenades, the grenade / frag launchers, the
    /// concussion gun's blast charge).
    Grenade,
}

impl AmmoType {
    /// The five ammunition classes in declaration order — the exhaustive sweep
    /// the compatibility tests iterate (the
    /// [`DamageType::ALL`](super::DamageType::ALL) precedent).
    pub const ALL: [Self; 5] = [
        Self::Slug,
        Self::Cell,
        Self::Flask,
        Self::Canister,
        Self::Grenade,
    ];
}
