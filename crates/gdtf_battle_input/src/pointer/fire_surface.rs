//! Fire request construction from selected shooter and fire mode.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::FireRequested,
    fire::{MeleeQuery, MountedQuery},
    ganger::{Aiming, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::{FireActor, FireRefusal, Magazine, fire_refusal},
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

/// A shooter's firing state and the weapon they fire.
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
    /// Mounted probe used to prefer a manned mount over the carried gun.
    mounted:  MountedQuery<'w, 's>,
}

impl ShooterArms<'_, '_> {
    /// The magazine and handedness of the weapon this shooter fires, if it has one.
    fn firing_weapon(&self, shooter: Entity) -> Option<(&Magazine, &Handedness)> {
        self.wields
            .get(shooter)
            .ok()
            .and_then(|wields| {
                wields.firing_weapon(
                    |entity| self.mounted.get(entity).is_ok(),
                    |entity| self.melee.get(entity).is_ok(),
                )
            })
            .and_then(|weapon| self.weapons.get(weapon).ok())
    }
}

/// Why no shot could be built for this shooter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotRefusal {
    /// The entity holds none of the parts a shooter needs.
    NotAShooter,
    /// The shooter wields no weapon that fires.
    NoFiringWeapon,
    /// The shooter is not alive.
    NotAlive,
    /// The shooter cannot afford the fire mode's time units.
    Unaffordable,
    /// The magazine holds no rounds.
    MagazineEmpty,
    /// The target cell-level is off the battle grid.
    OutOfBounds,
    /// Too few hands are free for the weapon.
    NotEnoughHands,
}

impl ShotRefusal {
    /// Mirror the sim's own reason.
    #[must_use]
    pub const fn from_sim(refusal: FireRefusal) -> Self {
        match refusal {
            FireRefusal::NotAlive => Self::NotAlive,
            FireRefusal::Unaffordable => Self::Unaffordable,
            FireRefusal::MagazineEmpty => Self::MagazineEmpty,
            FireRefusal::OutOfBounds => Self::OutOfBounds,
            FireRefusal::NotEnoughHands => Self::NotEnoughHands,
        }
    }
}

/// The fire request this shooter can take at `target`, or the reason it cannot.
///
/// # Errors
/// Answers the sim's own reason when the shot is not allowed.
pub fn try_fire_request(
    shooter: Entity,
    target: CellLevel,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    arms: &ShooterArms,
) -> Result<FireRequested, ShotRefusal> {
    let Ok((life, tu, tu_max, aiming, injuries)) = arms.shooters.get(shooter) else {
        return Err(ShotRefusal::NotAShooter);
    };
    let Some((magazine, handedness)) = arms.firing_weapon(shooter) else {
        return Err(ShotRefusal::NoFiringWeapon);
    };
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

    if let Some(refusal) = fire_refusal(&actor, fire_mode, target_cell, target_level, tuning) {
        return Err(ShotRefusal::from_sim(refusal));
    }

    Ok(FireRequested::new(
        shooter,
        **fire_mode,
        target_cell,
        target_level,
    ))
}
