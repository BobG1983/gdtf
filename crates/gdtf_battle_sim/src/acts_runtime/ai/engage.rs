//! The ENGAGE evaluation — the brain's weapon-resolution [`SystemParam`] bundle and
//! the per-target `can_see` ∧ `can_fire` ∧ `can_engage` gate over the opposing rows.

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
    fire::{MeleeQuery, WieldsQuery},
    ganger::Tu,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{FireMode, FireModeSpec, Handedness},
};

/// The brain's **weapon-resolution** [`SystemParam`] bundle — the three queries the
/// engage path keys `enemy → Wields → the RANGED weapon entity` through, grouped into
/// one param so [`enemy_ai_turn`] stays under Bevy's 16-param `SystemParam`-tuple arity
/// (GTW-505 added the `melee` probe, which pushed the flat list to 17 — the GTW-461
/// `ActPacing` bundling precedent).
///
/// Each is the existing query type ([`WieldsQuery`] / the weapon-stat query / the GTW-505
/// [`MeleeQuery`] marker probe); the bundle is a transparent grouping of existing
/// world-state queries, not a wrapped domain scalar.
#[derive(SystemParam)]
pub struct WeaponLookup<'w, 's> {
    /// The wielded-weapon relationship — `&Wields` on the enemy ganger.
    wields:  WieldsQuery<'w, 's>,
    /// The ranged weapon-stat columns read off the resolved weapon entity.
    weapons: Query<'w, 's, (&'static Magazine, &'static FireMode, &'static Handedness)>,
    /// GTW-505 C5: the melee-weapon marker probe — `ranged_weapon` filters the wielded
    /// weapon against it so the enemy's melee weapon is never engaged as its gun.
    melee:   MeleeQuery<'w, 's>,
}

impl WeaponLookup<'_, '_> {
    /// Resolve `enemy → Wields → the RANGED weapon entity` and read its `(Magazine,
    /// FireMode, Handedness)` — excluding the melee weapon the enemy also wields (GTW-505
    /// C5), the same ranged-filtered resolution `dispatch_fire` / `fire()` use. `None`
    /// when the enemy wields no ranged weapon or its weapon entity is missing.
    pub(super) fn ranged(&self, enemy: Entity) -> Option<(&Magazine, &FireMode, &Handedness)> {
        let weapon = self
            .wields
            .get(enemy)
            .ok()?
            .ranged_weapon(|entity| self.melee.get(entity).is_ok())?;
        self.weapons.get(weapon).ok()
    }
}

/// The per-target ENGAGE gate — collect every opposing row `enemy` can engage with its
/// resolved `weapon` (a target is engageable iff [`can_see`] (real per-pair LOS) ∧
/// [`can_fire`] (the shared fire guard, incl. the GTW-443 hand-count clause) ∧
/// [`can_engage`] (the SHARED `¬Reject` arc verdict)) — extracted verbatim from
/// [`enemy_ai_turn`](super::brain::enemy_ai_turn)'s per-enemy loop so that system stays
/// under the line cap. Pure over the snapshot rows: no emission, no mutation.
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
        // can_see (the ONE LOS truth) — conscious observer, in range, clear LOS.
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
        // can_fire (the shared fire guard) — alive, affordable, loaded, in-bounds.
        let actor = FireActor {
            life: &enemy.life,
            tu: &enemy.tu,
            tu_max: &enemy.tu_max,
            aiming: &enemy.aiming,
            magazine: &magazine,
            handedness,
            hands_available: enemy.hands,
        };
        if !can_fire(&actor, &mode, target_cell, target_level, tuning) {
            continue;
        }
        // can_engage (the SHARED ¬Reject arc verdict) — load-bearing for termination:
        // it guarantees the dispatcher will spend TU rather than silently reject.
        if !can_engage(
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
