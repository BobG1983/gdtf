//! Weapon emplacements: vacant/occupied state and mount/dismount.

pub mod relationship;
pub mod state;
pub mod toggle;

#[cfg(test)]
mod test;

pub use relationship::{Mounted, MountedBy};
pub use state::{
    EmplacementEntrySides, EmplacementFacing, EmplacementManned, EmplacementState, EnteredFrom,
    MountedWeaponEntity, MountedWeaponKey,
};
pub use toggle::{EmplacementTogglePlugin, SetEmplacement, apply_emplacement_toggle};
