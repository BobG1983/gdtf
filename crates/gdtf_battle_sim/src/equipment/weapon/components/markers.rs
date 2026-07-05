//! Unit entity markers — the [`Weapon`] armed tag and the emplacement's
//! [`MountedWeapon`] tag.

use bevy::prelude::Component;

/// The **`Weapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking an
/// entity as armed (GTW-200's user-corrected model).
///
/// The weapon's stats are **not** packed inside this — each is its own sibling
/// `#[derive(Component)]` newtype on the same entity (`BaseSpread`, `Accuracy`,
/// `Kickback`, `FatalBias`, `WeaponDamage`, `WeaponPunch`, `WeaponShred`,
/// `DamageType`, the [`crate::magazine::Magazine`] grouping (its `MagazineSize` /
/// `ReloadTu` / `LoadedRounds` leaves), [`FireMode`](crate::weapon::FireMode), `Stable`),
/// spawned together via [`WeaponBundle`](crate::weapon::WeaponBundle). Because the stats are direct
/// components, the combat act (E4.5 `fire()`) is a proper query-based Bevy system
/// with NO `&mut World` indirection: it queries the individual stat components off
/// the ganger entity (or assembles a transient [`WeaponStats`](crate::weapon::WeaponStats)
/// borrow-view from them). There is **no** `Weapon::new` and no packed data struct
/// any more — the stats live as components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;

/// The **`MountedWeapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking a
/// RANGED weapon entity as the bolted-down gun of a **weapon emplacement** the occupant is
/// currently manning (GTW-543, child GTW-41c of the emplacements epic GTW-41).
///
/// A mounted weapon is a NORMAL ranged weapon — it carries the ranged [`Weapon`] marker + the
/// full stat set + a [`Magazine`](crate::magazine::Magazine), spawned via
/// [`WeaponBundle`](crate::weapon::WeaponBundle) — that ALSO carries THIS marker. It is spawned onto
/// the OCCUPANT (related via [`WieldedBy`](crate::weapon::WieldedBy)) when a ganger ENTERS an
/// emplacement, and despawned when it EXITS, so it exists only for the duration of occupancy.
///
/// While present it is the shooter's PREFERRED ranged weapon: the fire path resolves
/// `ganger → Wields → the weapon entity` through
/// [`Wields::mounted_weapon`](crate::weapon::Wields::mounted_weapon) FIRST (a mounted-marked entity),
/// falling back to [`Wields::ranged_weapon`](crate::weapon::Wields::ranged_weapon) (the ganger's own
/// carried gun) when no mount is present — so a manning ganger fires the heavy mounted gun
/// instead of its side-arm, and reverts to its own weapon on exit. It is the emplacement
/// analogue of the [`MeleeWeapon`](crate::weapon::MeleeWeapon) marker: a distinguishing tag on a
/// wielded weapon entity that a keyed [`Wields`](crate::weapon::Wields) accessor selects.
///
/// `Default` lets a spawn path derive it; the marker carries no data.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MountedWeapon;
