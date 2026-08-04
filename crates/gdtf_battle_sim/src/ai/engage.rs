//! Which targets can this enemy currently fire at?

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Query},
};

use super::{
    decide::AiTarget,
    snapshot::{GangerRow, row_cell_level},
};
use crate::{
    acts::can_engage,
    cover::CoverLedger,
    fire::{MeleeQuery, MountedQuery, WieldsQuery},
    ganger::Tu,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{FireMode, FireModeSpec, Handedness},
};

/// Lookup for the weapon an enemy is currently firing.
#[derive(SystemParam)]
pub struct WeaponLookup<'w, 's> {
    wields:  WieldsQuery<'w, 's>,
    weapons: Query<'w, 's, (&'static Magazine, &'static FireMode, &'static Handedness)>,
    melee:   MeleeQuery<'w, 's>,
    mounted: MountedQuery<'w, 's>,
}

impl WeaponLookup<'_, '_> {
    /// Magazine, fire mode, and handedness for the enemy's firing weapon.
    pub(super) fn firing(&self, enemy: Entity) -> Option<(&Magazine, &FireMode, &Handedness)> {
        let weapon = self.wields.get(enemy).ok()?.firing_weapon(
            |entity| self.mounted.get(entity).is_ok(),
            |entity| self.melee.get(entity).is_ok(),
        )?;
        self.weapons.get(weapon).ok()
    }
}

/// Targets the enemy can see, afford to fire at, and engage right now.
#[expect(
    clippy::too_many_arguments,
    reason = "the gate borrows the brain's own reads (the enemy + target snapshot rows, \
              the resolved weapon triple + its fire cost, the four read grids + tuning the \
              can_see/can_fire/can_engage gates need, and the is_dead corpse predicate); \
              each is a distinct borrow mirroring enemy_ai_turn's own argument-count \
              carve-out — bundling would only hide the reads"
)]
pub(super) fn engageable_targets(
    enemy: &GangerRow,
    targets: &[GangerRow],
    weapon: (Magazine, FireModeSpec, Handedness),
    fire_cost: Tu,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: &impl Fn(Entity) -> bool,
) -> Vec<AiTarget> {
    let (magazine, mode, handedness) = weapon;
    let enemy_cell = enemy.position.cell();
    let enemy_cell_level = row_cell_level(&enemy.position);
    let observer = Observer {
        position:         &enemy.position,
        stance:           &enemy.stance,
        facing:           &enemy.facing,
        stair_eye_offset: occupancy.stair_eye_offset_at(&enemy_cell_level),
        peek_offset:      PeekOffset::default(),
    };
    let mut engageable: Vec<AiTarget> = Vec::new();
    for target_row in targets {
        let target_cell = target_row.position.cell();
        let target_level = target_row.position.level();
        let target = Target {
            position: &target_row.position,
            stance:   &target_row.stance,
        };
        if !*can_see(
            &observer,
            &target,
            enemy.life,
            tuning.view_range,
            occupancy,
            surface,
            cover,
            tuning,
            is_dead,
        ) {
            continue;
        }
        let actor = FireActor {
            life: &enemy.life,
            tu: &enemy.tu,
            tu_max: &enemy.tu_max,
            aiming: &enemy.aiming,
            magazine: &magazine,
            handedness,
            hands_available: enemy.hands,
        };
        if !*can_fire(&actor, &mode, target_cell, target_level, tuning) {
            continue;
        }
        if !*can_engage(
            *enemy.facing,
            enemy_cell,
            target_cell,
            enemy.tu,
            fire_cost,
            tuning,
        ) {
            continue;
        }
        engageable.push(AiTarget::new(target_row.entity, target_cell, target_level));
    }
    engageable
}
