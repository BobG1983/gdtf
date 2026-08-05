//! System param for the weapon a ganger currently fires.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Query, With},
};

use super::{
    Silenced,
    silenced::{ShotSilenced, shooter_weapon_silenced},
};
use crate::{acts::WeaponProbes, fire::WieldsQuery};

/// Wield and weapon-tag lookups for the weapon a ganger fires.
#[derive(SystemParam)]
pub struct FiringWeapon<'w, 's> {
    wields:   WieldsQuery<'w, 's>,
    probes:   WeaponProbes<'w, 's>,
    silenced: Query<'w, 's, (), With<Silenced>>,
}

impl FiringWeapon<'_, '_> {
    /// The weapon entity this shooter would fire, if any.
    #[must_use]
    pub fn of(&self, shooter: Entity) -> Option<Entity> {
        self.wields.get(shooter).ok().and_then(|wields| {
            wields.firing_weapon(
                |entity| self.probes.mounted.get(entity).is_ok(),
                |entity| self.probes.melee.get(entity).is_ok(),
            )
        })
    }

    /// Whether that weapon is silenced.
    #[must_use]
    pub fn silenced(&self, shooter: Entity) -> ShotSilenced {
        shooter_weapon_silenced(
            shooter,
            &self.wields,
            &self.probes.mounted,
            &self.probes.melee,
            &self.silenced,
        )
    }
}
