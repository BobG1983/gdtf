//! The per-emplacement-entity **state components** — [`EmplacementState`] (the
//! vacant/occupied toggle), [`EmplacementOccupant`] (which ganger is manning it), and
//! [`MountedWeaponKey`] (the registry key of the bolted-down gun). GTW-543, child GTW-41c
//! of the emplacements epic GTW-41.
//!
//! These mirror the door-precedent state pair
//! ([`OpenState`](crate::terrain::openable::OpenState) /
//! [`OpenableBlocking`](crate::terrain::openable::OpenableBlocking), GTW-503): an
//! emplacement is a stateful terrain piece a ganger enters and exits, so its live state
//! lives on the spawned terrain entity, read + flipped by the
//! [`apply_emplacement_toggle`](super::apply_emplacement_toggle) system.

use bevy::prelude::{Component, Deref, Entity};

use crate::weapon::WeaponName;

/// The vacant/occupied state of a **weapon emplacement** — a heavy mounted-gun position
/// (GTW-543).
///
/// A NAMED CLOSED enum [`Component`] (no-bare-types: an emplacement's occupancy state is a
/// domain value, NOT a bare `bool` — a manned emplacement reads `EmplacementState::Occupied`,
/// never `true`). Attached at terrain-entity spawn to every
/// [`TerrainSimKind::Emplacement`](crate::terrain::def::TerrainSimKind::Emplacement) piece,
/// DEFAULT [`Vacant`](EmplacementState::Vacant) (an emplacement starts unmanned).
///
/// The toggle ([`apply_emplacement_toggle`](super::apply_emplacement_toggle)) flips this on
/// the [`SetEmplacement`](super::SetEmplacement) message:
///
/// - [`Vacant`](EmplacementState::Vacant) — unmanned. The emplacement still stands as a
///   cover-like structure (its `BlocksPathfinding` / `BlocksVision` / `CoverLedger` entry
///   persist regardless of occupancy — an empty emplacement is still cover).
/// - [`Occupied`](EmplacementState::Occupied) — a ganger is manning it (recorded in
///   [`EmplacementOccupant`]). While occupied, the occupant's published silhouette band is
///   FORCED to [`HeightBand::High`](crate::cover::HeightBand::High) so it reads as HIGH cover,
///   and the mounted gun is steadied by the
///   [`EmplacementStability`](crate::stability::EmplacementStability) seam (Phase 2).
///
/// `Default` is [`Vacant`](EmplacementState::Vacant): a freshly-spawned emplacement is
/// unmanned.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EmplacementState {
    /// The emplacement is unmanned — no occupant, no band-forcing, no mounted gun spawned.
    /// The spawn default.
    #[default]
    Vacant,
    /// The emplacement is manned — a ganger (recorded in [`EmplacementOccupant`]) operates it.
    Occupied,
}

/// Whether a weapon emplacement is **manned** — the answer [`EmplacementState::is_occupied`]
/// returns (a ganger is currently operating it).
///
/// A named newtype over `bool` (no-bare-types: an emplacement's occupancy is a domain fact, not
/// a bare boolean — a manned emplacement reads `EmplacementManned(true)`). Private inner +
/// derived [`Deref`] (house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementManned(bool);

impl EmplacementManned {
    /// Build a manned answer from its boolean state.
    #[must_use]
    pub const fn new(manned: bool) -> Self {
        Self(manned)
    }
}

impl EmplacementState {
    /// Whether this state is [`Occupied`](EmplacementState::Occupied) — a ganger is manning
    /// the emplacement.
    #[must_use]
    pub const fn is_occupied(self) -> EmplacementManned {
        EmplacementManned::new(matches!(self, Self::Occupied))
    }
}

/// The ganger [`Entity`] currently manning an **occupied** emplacement (GTW-543) — recorded
/// on the emplacement entity while it is [`Occupied`](EmplacementState::Occupied), so the exit
/// act (and Phase-2 mounted-weapon despawn) knows whose band to restore.
///
/// A single-field tuple [`Component`] wrapping an [`Entity`]. The inner [`Entity`] is
/// **Bevy-mandated framework plumbing** — an entity handle, NOT a wrapped domain scalar (the
/// no-bare-types framework carve-out) — read through the derived [`Deref`], constructed via
/// [`new`](EmplacementOccupant::new). Present ONLY while the emplacement is occupied: the
/// toggle inserts it on occupy and removes it on vacate (mirroring how
/// [`OpenableBlocking`](crate::terrain::openable::OpenableBlocking) records the door's closed
/// band, but transient rather than persistent).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmplacementOccupant(Entity);

impl EmplacementOccupant {
    /// Record the ganger `entity` manning an emplacement — the occupant recorded on occupy.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// The registry KEY of the mounted weapon bolted to a **weapon emplacement** (GTW-543) —
/// recorded on the emplacement entity at spawn so the enter act (Phase 2) can resolve it
/// against the [`WeaponRegistry`](crate::weapon::WeaponRegistry) and spawn the mounted gun on
/// the occupant WITHOUT re-reading the [`TerrainDef`](crate::terrain::def::TerrainDef).
///
/// A single-field tuple [`Component`] wrapping a [`WeaponName`] (no-bare-types: the mounted
/// gun's identity is a domain value, exposed read-only through the derived [`Deref`], inner
/// private, constructed via [`new`](MountedWeaponKey::new)). The band-record analogue of
/// [`OpenableBlocking`](crate::terrain::openable::OpenableBlocking): a PERSISTENT record set
/// once at spawn from the def's
/// [`Emplacement::mounted_weapon`](crate::terrain::def::TerrainSimKind::Emplacement) key.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MountedWeaponKey(WeaponName);

impl MountedWeaponKey {
    /// Build the mounted-weapon key record from the def's
    /// [`Emplacement::mounted_weapon`](crate::terrain::def::TerrainSimKind::Emplacement)
    /// [`WeaponName`].
    #[must_use]
    pub const fn new(key: WeaponName) -> Self {
        Self(key)
    }
}

/// The spawned mounted-weapon [`Entity`] currently WIELDED BY an **occupied** emplacement's
/// occupant (GTW-543, child GTW-41c) — recorded on the emplacement entity while it is
/// [`Occupied`](EmplacementState::Occupied), so the exit act can DESPAWN exactly the weapon it
/// spawned on enter (reverting the occupant to its own carried gun).
///
/// A single-field tuple [`Component`] wrapping an [`Entity`]. The inner [`Entity`] is
/// **Bevy-mandated framework plumbing** — an entity handle, NOT a wrapped domain scalar (the
/// no-bare-types framework carve-out) — read through the derived [`Deref`], constructed via
/// [`new`](MountedWeaponEntity::new). Present ONLY while the emplacement is occupied AND its
/// [`MountedWeaponKey`] resolved to a spawnable weapon: the occupy path spawns the mounted weapon
/// (related to the occupant via [`WieldedBy`](crate::weapon::WieldedBy) + the
/// [`MountedWeapon`](crate::weapon::MountedWeapon) marker) and records its entity here; the vacate
/// path despawns that entity and removes this record. Distinct from [`EmplacementOccupant`] (which
/// records the manning GANGER); this records the spawned GUN, so the despawn targets the exact
/// mounted weapon and not the occupant's own carried gun.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MountedWeaponEntity(Entity);

impl MountedWeaponEntity {
    /// Record the mounted-weapon `entity` spawned on the occupant when it manned the emplacement.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }
}
