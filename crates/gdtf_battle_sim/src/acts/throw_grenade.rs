//! Throw grenade: spend TU, march arc, resolve blast.

use bevy::{
    ecs::query::With,
    prelude::{MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::request::{ThrowGrenadeRequested, ThrowResolved},
    effects::attachments::WeaponBraceBonus,
    fire::{BattleGrids, BlastFootprint, MeleeQuery, StruckBodies, resolve_blast},
    ganger::{Luck, Position, Tu},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::Magazine,
    march::{MarchResult, march_arc},
    resolve_and_apply::{ShotSource, WoundRoll},
    rng::{InjuryRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, HitType, Kickback,
        Stable, TrajectoryStyle, WeaponDamage, WeaponPunch, WeaponShred, WeaponStats, WieldedBy,
        Wields,
    },
};

type ThrowWeaponQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static BaseSpread,
        &'static Accuracy,
        &'static Kickback,
        &'static FatalBias,
        &'static WeaponDamage,
        &'static WeaponPunch,
        &'static WeaponShred,
        &'static DamageType,
        &'static Stable,
        Option<&'static WeaponBraceBonus>,
        Option<&'static DotProfile>,
        &'static TrajectoryStyle,
        &'static FireMode,
        &'static mut Magazine,
    ),
    With<WieldedBy>,
>;

/// Thrower query plus optional injury content resources.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ThrowActor<'w, 's> {
    pub thrower:  Query<'w, 's, (&'static Position, &'static Luck, &'static mut Tu)>,
    pub tuning:   Option<Res<'w, CombatTuning>>,
    pub tables:   Option<Res<'w, InjuryTables>>,
    pub registry: Option<Res<'w, InjuryRegistry>>,
}

/// Grids and RNGs used while resolving a throw.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ThrowWorld<'w> {
    pub occupancy:    Res<'w, crate::occupancy::OccupancyGrid>,
    pub surface:      Res<'w, SurfaceGrid>,
    pub cover:        ResMut<'w, crate::cover::CoverLedger>,
    pub slab:         ResMut<'w, SlabLedger>,
    pub brace_cells:  Res<'w, BraceStairCells>,
    pub shot_rng:     ResMut<'w, ShotRng>,
    pub severity_rng: ResMut<'w, SeverityRng>,
    pub injury_rng:   ResMut<'w, InjuryRng>,
}

struct GrenadeStats {
    base_spread: BaseSpread,
    accuracy:    Accuracy,
    kickback:    Kickback,
    fatal_bias:  FatalBias,
    damage:      WeaponDamage,
    punch:       WeaponPunch,
    shred:       WeaponShred,
    damage_type: DamageType,
    stable:      Stable,
    brace_bonus: Option<WeaponBraceBonus>,
    dot:         Option<DotProfile>,
    hit_type:    HitType,
}

impl GrenadeStats {
    const fn stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
            brace_bonus: self.brace_bonus.as_ref(),
            dot:         self.dot.as_ref(),
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the throw needs its request reader + the disjoint thrower-actor / weapon / \
              target / wears / pieces queries + the grouped world-grid & RNG bundle + the \
              ThrowResolved signal writer — each a distinct Bevy SystemParam the arc march + \
              blast fold reads; the actor + world bundles already group the resources to stay \
              under Bevy's 16-param limit"
)]
/// Process throw requests for arc weapons with ammo.
pub fn dispatch_throw_grenade(
    mut requests: MessageReader<ThrowGrenadeRequested>,
    mut actor: ThrowActor,
    mut weapons: ThrowWeaponQuery,
    wields: Query<&Wields>,
    melee: MeleeQuery,
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

        let Some(weapon_entity) = wields
            .get(request.thrower)
            .ok()
            .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
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
        )) = weapons.get_mut(weapon_entity)
        else {
            continue;
        };

        if !*trajectory.is_arc() || *magazine.is_empty() {
            continue;
        }

        let cost = Tu::new(*tuning.throw_tu);
        if **thrower_tu < *cost {
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

        spend_tu(&mut thrower_tu, cost);
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
