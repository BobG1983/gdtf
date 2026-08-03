//! Whether the shooter's firing weapon is silenced.

use bevy::prelude::{Deref, Entity, Query, With};

use super::Silenced;
use crate::fire::{MeleeQuery, MountedQuery, WieldsQuery};

/// True if the shot should not trigger reaction fire from noise.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotSilenced(bool);

/// Resolve the shooter's firing weapon and check for [`Silenced`].
#[must_use]
pub fn shooter_weapon_silenced(
    shooter: Entity,
    wields: &WieldsQuery,
    mounted: &MountedQuery,
    melee: &MeleeQuery,
    silenced: &Query<(), With<Silenced>>,
) -> ShotSilenced {
    ShotSilenced(
        wields
            .get(shooter)
            .ok()
            .and_then(|w| {
                w.firing_weapon(
                    |entity| mounted.get(entity).is_ok(),
                    |entity| melee.get(entity).is_ok(),
                )
            })
            .is_some_and(|weapon| silenced.get(weapon).is_ok()),
    )
}
