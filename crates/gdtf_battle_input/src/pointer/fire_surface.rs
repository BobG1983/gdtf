//! Fire request construction from selected shooter and fire mode.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::FireRequested,
    fire::MeleeQuery,
    ganger::{Aiming, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::{FireActor, Magazine, can_fire},
    prelude::{CellLevel, LifeState, Tu},
    tuning::CombatTuning,
    weapon::{Handedness, WieldedBy, Wields},
};

use crate::SelectedFireMode;

/// Query data needed to decide whether a shooter can fire.
pub type ShooterFireData<'a> = (
    &'a LifeState,
    &'a Tu,
    &'a TuMax,
    &'a Aiming,
    Option<&'a InflictedInjuries>,
);

/// Magazine and handedness on a wielded weapon entity.
pub type WeaponMagazine<'a> = (&'a Magazine, &'a Handedness);

/// A shooter's firing state and the ranged weapon they wield.
#[derive(SystemParam)]
pub struct ShooterArms<'w, 's> {
    /// Life, time units, aiming, and injuries of the shooter.
    shooters: Query<'w, 's, ShooterFireData<'static>>,
    /// Weapons each shooter holds.
    wields:   Query<'w, 's, &'static Wields>,
    /// Magazine and handedness of a wielded weapon.
    weapons:  Query<'w, 's, WeaponMagazine<'static>, With<WieldedBy>>,
    /// Melee probe used to skip melee weapons when picking the ranged one.
    melee:    MeleeQuery<'w, 's>,
}

impl ShooterArms<'_, '_> {
    /// The shooter's ranged weapon magazine and handedness, if they hold one.
    fn ranged_weapon(&self, shooter: Entity) -> Option<(&Magazine, &Handedness)> {
        self.wields
            .get(shooter)
            .ok()
            .and_then(|wields| wields.ranged_weapon(|entity| self.melee.get(entity).is_ok()))
            .and_then(|weapon| self.weapons.get(weapon).ok())
    }
}

#[must_use]
pub(crate) fn try_fire_request(
    shooter: Entity,
    target: CellLevel,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    arms: &ShooterArms,
) -> Option<FireRequested> {
    let Ok((life, tu, tu_max, aiming, injuries)) = arms.shooters.get(shooter) else {
        return None;
    };
    let (magazine, handedness) = arms.ranged_weapon(shooter)?;
    let hands_available =
        injuries.map_or_else(HandsAvailable::default, InflictedInjuries::hands_available);
    let actor = FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness: *handedness,
        hands_available,
    };
    let (target_cell, target_level) = target.split();

    if !*can_fire(&actor, fire_mode, target_cell, target_level, tuning) {
        return None;
    }

    Some(FireRequested::new(
        shooter,
        **fire_mode,
        target_cell,
        target_level,
    ))
}
