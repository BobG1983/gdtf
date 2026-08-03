//! Fire request construction from selected shooter and fire mode.

use bevy::prelude::*;
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

#[expect(
    clippy::too_many_arguments,
    reason = "GTW-505 C5: the ranged-weapon resolution adds the MeleeWeapon marker probe on top of \
              the GTW-323 slice-3 Wields + weapon-magazine queries, so the gun's magazine resolves \
              excluding the melee weapon the ganger also wields"
)]
#[must_use]
pub(crate) fn try_fire_request(
    shooter: Entity,
    target: CellLevel,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    shooters: &Query<ShooterFireData>,
    wields: &Query<&Wields>,
    weapons: &Query<WeaponMagazine, With<WieldedBy>>,
    melee: &MeleeQuery,
) -> Option<FireRequested> {
    let Ok((life, tu, tu_max, aiming, injuries)) = shooters.get(shooter) else {
        return None;
    };
    let (magazine, handedness) = wields
        .get(shooter)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| weapons.get(weapon).ok())?;
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
