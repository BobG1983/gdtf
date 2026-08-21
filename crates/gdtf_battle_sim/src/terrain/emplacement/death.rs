//! Death frees the seat: the body stays on the mount cell, the emplacement goes vacant.

use bevy::prelude::{Changed, Commands, Query};

use super::{
    relationship::Mounted,
    state::{EmplacementState, MountedWeaponEntity},
    vacate::clear_seat,
};
use crate::ganger::LifeState;

/// Give the seat up for every mounted ganger killed this frame, writing no `Position`.
pub(crate) fn clear_seat_on_death(
    mut commands: Commands,
    killed: Query<(&LifeState, &Mounted), Changed<LifeState>>,
    mut seats: Query<(&mut EmplacementState, Option<&MountedWeaponEntity>)>,
) {
    for (life, mounted) in &killed {
        if !matches!(life, LifeState::Dead) {
            continue;
        }
        let Some(seat) = mounted.emplacement() else {
            continue;
        };
        let Ok((mut state, mount)) = seats.get_mut(seat) else {
            continue;
        };
        clear_seat(&mut commands, seat, &mut state, mount);
    }
}
