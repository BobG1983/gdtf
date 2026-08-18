//! Plan an aim-on act before a shot, or a crouch when nothing else applies.

use bevy::prelude::Entity;

use super::{
    engage::{WeaponLookup, engageable_targets},
    params::AiPlanningGrids,
    snapshot::GangerRow,
};
use crate::{
    acts::{AimRequest, SetAimingRequested, SetStanceRequested},
    ganger::{Aiming, StanceKind},
    magazine::mode_tu_cost,
    posture::can_set_stance,
    tu::can_spend_tu,
};

// Aim on when a target is engageable at the aimed fire cost and we are not already aiming.
pub(super) fn plan_aim(
    enemy: &GangerRow,
    targets: &[GangerRow],
    weapons: &WeaponLookup,
    grids: &AiPlanningGrids,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<SetAimingRequested> {
    if *enemy.aiming {
        return None;
    }
    let (magazine, fire_mode, handedness) = weapons.firing(enemy.entity)?;
    let mode = fire_mode.single();
    let aimed = Aiming::new(true);
    let aimed_cost = mode_tu_cost(&mode, &enemy.tu_max, &aimed, grids.tuning());
    if !*can_spend_tu(&enemy.tu, aimed_cost) {
        return None;
    }
    let engageable = engageable_targets(
        enemy,
        targets,
        (*magazine, mode, *handedness),
        aimed_cost,
        grids.march(),
        grids.tuning(),
        is_dead,
    );
    (!engageable.is_empty()).then(|| SetAimingRequested::new(enemy.entity, AimRequest::new(true)))
}

// Crouch when still standing and the stance change is legal and affordable.
pub(super) fn plan_crouch(
    enemy: &GangerRow,
    grids: &AiPlanningGrids,
) -> Option<SetStanceRequested> {
    (*can_set_stance(
        &enemy.stance,
        StanceKind::Crouching,
        &enemy.tu,
        &grids.tuning().stance_change_tu,
    ))
    .then(|| SetStanceRequested::new(enemy.entity, StanceKind::Crouching))
}
