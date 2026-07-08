//! The melee-only per-stat **components** — the [`Reach`] newtype and the
//! [`MeleeWeapon`] unit marker (GTW-505, child GTW-37a of the GTW-37 melee epic).
//!
//! These are the components that DISTINGUISH a melee weapon entity from a ranged one:
//! a melee weapon REUSES every shared ranged damage newtype verbatim (the
//! [`WeaponName`](super::super::WeaponName) / [`WeaponDamage`](super::super::WeaponDamage)
//! group / [`FatalBias`](super::super::FatalBias) / [`Handedness`](super::super::Handedness)),
//! adds the melee-only [`Reach`] + [`FightMode`](super::FightMode), and carries the
//! [`MeleeWeapon`] marker INSTEAD OF the ranged [`Weapon`](super::super::Weapon) marker so a
//! ranged-weapon lookup can EXCLUDE it (GTW-505 C5 — see
//! [`Wields::ranged_weapon`](super::super::Wields::ranged_weapon)).

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A melee weapon's **reach** — how many cells away a strike can land (GTW-505). A
/// knife/fist reaches the adjacent cell (`1`); a spear/polearm reaches further. The
/// GTW-506 opposed-Fight resolution will read this to gate which cells a melee strike
/// can target; this slice carries it as DATA only.
///
/// A weapon NUMBER (lives on the melee weapon, not in tuning), a small non-negative
/// cell count (`u16`, the band/count inner type). Private inner + derived [`Deref`];
/// `#[serde(transparent)]` parses a bare RON scalar (the
/// [`crate::tuning`] / GTW-200 house style), and [`Serialize`] so the editor's MELEE
/// mode (GTW-671) saves the field in the same bare-scalar schema it loads. A
/// `#[derive(Component)]` so it lives as a sibling component on the armed melee-weapon
/// entity.
///
/// `Default` ([`Reach::DEFAULT`] = `1`) is the DEFENSIBLE melee reach AND the
/// `bsn!`-spawn-seed sentinel (the GTW-322 convention): the `bsn!` spawn path seeds the
/// slot via `Default` before the authored `Reach::new(..)` overwrites it. Unlike the
/// ranged sentinels (which default to a NEUTRAL zero), `1` is BOTH a sensible default
/// AND the sentinel — a melee weapon authored WITHOUT a `reach:` field is a
/// reach-1 weapon (the `#[serde(default)]` on the spec field makes the field optional).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reach(u16);

impl Reach {
    /// The default melee reach — `1` cell (the adjacent-cell strike). The
    /// `#[serde(default)]` melee spec field falls back to this, and it doubles as the
    /// `bsn!` spawn-seed sentinel.
    pub const DEFAULT: Self = Self(1);

    /// Build a reach value from its cell count.
    #[must_use]
    pub const fn new(reach: u16) -> Self {
        Self(reach)
    }
}

impl Default for Reach {
    /// The defensible-default reach: `1` cell ([`Reach::DEFAULT`]).
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The **`MeleeWeapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking
/// an entity as a MELEE weapon (GTW-505), the melee mirror of the ranged
/// [`Weapon`](super::super::Weapon) marker.
///
/// A melee weapon entity carries this INSTEAD OF the ranged [`Weapon`](super::super::Weapon)
/// marker, so a ranged-weapon lookup can tell the two apart and EXCLUDE melee weapons
/// from ranged resolution (GTW-505 C5 — the zero-ranged-regression mechanism). A ganger
/// wields BOTH a ranged weapon entity (carrying [`Weapon`](super::super::Weapon)) AND a
/// melee weapon entity (carrying this marker), both related via
/// [`Wields`](super::super::Wields); the ranged-firing path resolves the ranged one via
/// [`Wields::ranged_weapon`](super::super::Wields::ranged_weapon) (which filters out
/// `MeleeWeapon`), and the GTW-506/507 melee path resolves this one via
/// [`Wields::melee_weapon`](super::super::Wields::melee_weapon).
///
/// The melee weapon's stats are NOT packed inside this — each is its own sibling
/// `#[derive(Component)]` newtype on the same entity (the shared
/// [`WeaponName`](super::super::WeaponName) / [`WeaponDamage`](super::super::WeaponDamage) /
/// [`WeaponPunch`](super::super::WeaponPunch) / [`WeaponShred`](super::super::WeaponShred) /
/// [`DamageType`](super::super::DamageType) / [`FatalBias`](super::super::FatalBias) /
/// [`Handedness`](super::super::Handedness), plus the melee-only [`Reach`] +
/// [`FightMode`](super::FightMode)), spawned together via
/// [`MeleeWeaponBundle`](super::MeleeWeaponBundle). `Default` lets the bundle derive
/// `Default` for the GTW-322 sentinel-seed spawn path.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeleeWeapon;
