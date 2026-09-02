//! Weapon emplacements: vacant/occupied state and mount/dismount.

mod death;
mod eject;
pub mod entry;
pub mod relationship;
pub mod state;
pub mod toggle;
mod vacate;

#[cfg(test)]
mod test;

pub(crate) use eject::eject_on_destroy;
pub use entry::{EntryCells, emplacement_entry_cells};
pub use relationship::{Mounted, MountedBy};
pub use state::{
    EmplacementEntrySides, EmplacementFacing, EmplacementManned, EmplacementState, EnteredFrom,
    MountedWeaponEntity, MountedWeaponKey,
};
pub use toggle::{EmplacementTogglePlugin, SetEmplacement, apply_emplacement_toggle};
pub(crate) use vacate::clear_seat;
