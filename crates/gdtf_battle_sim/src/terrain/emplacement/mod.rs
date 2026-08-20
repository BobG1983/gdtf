//! Weapon emplacements: vacant/occupied state and mount/dismount.

pub mod entry;
pub mod relationship;
pub mod state;
pub mod toggle;
mod vacate;

#[cfg(test)]
mod test;

pub use entry::{EntryCells, emplacement_entry_cells};
pub use relationship::{Mounted, MountedBy};
pub use state::{
    EmplacementEntrySides, EmplacementFacing, EmplacementManned, EmplacementState, EnteredFrom,
    MountedWeaponEntity, MountedWeaponKey,
};
pub use toggle::{EmplacementTogglePlugin, SetEmplacement, apply_emplacement_toggle};
pub(crate) use vacate::clear_seat;
