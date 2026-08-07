//! Throw grenade: spend TU, march arc, resolve blast.

use bevy::prelude::{MessageReader, MessageWriter};

use super::{
    cost::{can_throw_grenade, throw_grenade_tu_cost},
    params::{GrenadeStats, ThrowActor, ThrowWorld, ThrownArms},
};
use crate::{
    acts::request::{ThrowGrenadeRequested, ThrowResolved},
    fire::{BattleGrids, BlastFootprint, StruckBodies, resolve_blast},
    injuries::{InjuryRegistry, InjuryTables},
    march::{MarchResult, march_arc},
    resolve_and_apply::{ShotSource, WoundRoll},
    tu::spend_tu,
};

/// Process throw requests for arc weapons with ammo.
pub fn dispatch_throw_grenade(
    mut requests: MessageReader<ThrowGrenadeRequested>,
    mut actor: ThrowActor,
    mut arms: ThrownArms,
    mut bodies: StruckBodies,
    mut world: ThrowWorld,
    mut resolved: MessageWriter<ThrowResolved>,
) {
    let Some(tuning) = actor.tuning.as_deref() else {
        return;
    };
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = actor.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = actor.registry.as_deref().unwrap_or(&empty_registry);

    for request in requests.read() {
        let Ok((thrower_pos, thrower_luck, mut thrower_tu)) =
            actor.thrower.get_mut(request.thrower)
        else {
            continue;
        };
        let thrower_cell = **thrower_pos;
        let thrower_luck = *thrower_luck;

        let Some(weapon_entity) = arms
            .wields
            .get(request.thrower)
            .ok()
            .and_then(|w| w.ranged_weapon(|entity| arms.melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage,
            punch,
            shred,
            damage_type,
            stable,
            brace_bonus,
            dot,
            trajectory,
            fire_mode,
            mut magazine,
        )) = arms.weapons.get_mut(weapon_entity)
        else {
            continue;
        };

        if !*can_throw_grenade(*trajectory, &magazine, &thrower_tu, tuning) {
            continue;
        }

        let grenade = GrenadeStats {
            base_spread: *base_spread,
            accuracy:    *accuracy,
            kickback:    *kickback,
            fatal_bias:  *fatal_bias,
            damage:      *damage,
            punch:       *punch,
            shred:       *shred,
            damage_type: *damage_type,
            stable:      *stable,
            brace_bonus: brace_bonus.copied(),
            dot:         dot.copied(),
            hit_type:    fire_mode.single().hit_type,
        };

        if spend_tu(&mut thrower_tu, throw_grenade_tu_cost(tuning)).is_err() {
            continue;
        }
        magazine.spend_round();

        let MarchResult { at: landing, .. } =
            march_arc(thrower_cell, request.target, &world.surface, tuning);
        let mut grids = BattleGrids {
            occupancy:   &world.occupancy,
            surface:     &world.surface,
            cover:       &mut world.cover,
            slab:        &mut world.slab,
            brace_cells: &world.brace_cells,
        };
        resolve_blast(
            BlastFootprint {
                landing,
                thrower: thrower_cell,
                hit: grenade.hit_type,
            },
            ShotSource {
                weapon: grenade.stats(),
                luck:   thrower_luck,
            },
            &mut grids,
            &mut bodies,
            &mut world.shot_rng,
            &mut WoundRoll {
                tuning,
                severity_rng: &mut world.severity_rng,
                tables,
                registry,
                injury_rng: &mut world.injury_rng,
            },
        );

        resolved.write(ThrowResolved::new(landing, grenade.damage_type));
    }
}
