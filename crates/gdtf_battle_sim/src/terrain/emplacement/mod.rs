//! Weapon emplacements: vacant/occupied state and mount/dismount.

pub mod state;
pub mod toggle;

#[cfg(test)]
mod test;

pub use state::{
    EmplacementManned, EmplacementOccupant, EmplacementState, MountedWeaponEntity, MountedWeaponKey,
};
pub use toggle::{EmplacementTogglePlugin, SetEmplacement, apply_emplacement_toggle};
