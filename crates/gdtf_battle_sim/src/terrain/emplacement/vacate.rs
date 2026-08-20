//! Clearing a seat — the record every dismount drops, whatever caused it.

use bevy::prelude::{Commands, Entity};

use super::{
    relationship::MountedBy,
    state::{EmplacementState, EnteredFrom, MountedWeaponEntity},
};

/// Clear a seat: state back to vacant, occupant and entry records dropped, mount despawned.
/// Removing [`MountedBy`] clears the occupant's `Mounted`, its relationship target.
pub(crate) fn clear_seat(
    commands: &mut Commands,
    emplacement: Entity,
    state: &mut EmplacementState,
    mount: Option<&MountedWeaponEntity>,
) {
    *state = EmplacementState::Vacant;
    commands
        .entity(emplacement)
        .remove::<MountedBy>()
        .remove::<EnteredFrom>();
    if let Some(mount) = mount {
        commands.entity(**mount).despawn();
        commands.entity(emplacement).remove::<MountedWeaponEntity>();
    }
}
