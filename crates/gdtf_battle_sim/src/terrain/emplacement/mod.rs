//! The **weapon-emplacement** (mounted-gun position) layer (GTW-543, child GTW-41c of the
//! emplacements epic GTW-41).
//!
//! An emplacement is a cover-like smashable STRUCTURE (a
//! [`TerrainSimKind::Emplacement`](crate::terrain::def::TerrainSimKind::Emplacement) piece
//! resolved through the cover path, seeded into the
//! [`CoverLedger`](crate::cover::CoverLedger) with a `BlocksPathfinding` +
//! `BlocksVision` footprint) that ALSO carries a stateful enter/exit lifecycle a ganger
//! drives, modelled on the door precedent (GTW-315/503):
//!
//! - [`EmplacementState`] — the vacant/occupied state [`Component`](bevy::prelude::Component)
//!   attached at spawn to every emplacement (default [`Vacant`](EmplacementState::Vacant)).
//!   [`EmplacementOccupant`] — the ganger manning it while occupied (transient — present only
//!   while [`Occupied`](EmplacementState::Occupied)). [`MountedWeaponKey`] — the bolted-gun
//!   registry key, recorded at spawn.
//! - [`SetEmplacement`] — the occupy/vacate MESSAGE the enter/exit context acts write +
//!   [`apply_emplacement_toggle`] — the system that flips the state, records/drops the
//!   occupant, and forces the occupant's silhouette band to
//!   [`HeightBand::High`](crate::cover::HeightBand::High) while occupied (so it reads as HIGH
//!   cover), restoring it from stance on vacate. [`EmplacementTogglePlugin`] wires both.
//!
//! ## Phase split
//!
//! GTW-543 Phase 1 (this module) is the sim-terrain + enter/exit STATE core: the new
//! `TerrainSimKind::Emplacement` kind, the spawn (a cover-like structure carrying the
//! enter/exit components), and the band-forcing toggle. The mounted-weapon SPAWN/DESPAWN on
//! the occupant + the `EmplacementStability` engage + the `MountedWeapon`-prefer ranged read
//! are **Phase 2** — the [`apply_emplacement_toggle`] system leaves a clear seam (the
//! occupant + key records are exactly what Phase 2 reads).

pub mod state;
pub mod toggle;

#[cfg(test)]
mod test;

pub use state::{EmplacementOccupant, EmplacementState, MountedWeaponEntity, MountedWeaponKey};
pub use toggle::{EmplacementTogglePlugin, SetEmplacement, apply_emplacement_toggle};
