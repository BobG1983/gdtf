//! System params and the frozen grenade stats used while resolving a throw.

use bevy::{
    ecs::{query::With, system::SystemParam},
    prelude::{Query, Res, ResMut},
};

use crate::{
    effects::attachments::WeaponBraceBonus,
    fire::MeleeQuery,
    ganger::{Luck, Position, Tu},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::Magazine,
    occupancy::OccupancyGrid,
    rng::{InjuryRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, HitType, Kickback,
        Stable, TrajectoryStyle, WeaponDamage, WeaponPunch, WeaponShred, WeaponStats, WieldedBy,
        Wields,
    },
};

pub(super) type ThrowWeaponQuery<'world, 'state> = Query<
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
#[derive(SystemParam)]
pub struct ThrowActor<'w, 's> {
    /// Position, luck, and TU pool of each possible thrower.
    pub thrower:  Query<'w, 's, (&'static Position, &'static Luck, &'static mut Tu)>,
    /// Combat tuning, absent until content loads.
    pub tuning:   Option<Res<'w, CombatTuning>>,
    /// Injury tables, absent until content loads.
    pub tables:   Option<Res<'w, InjuryTables>>,
    /// Injury registry, absent until content loads.
    pub registry: Option<Res<'w, InjuryRegistry>>,
}

/// The arc weapon a thrower holds, and the probe that rules a melee weapon out.
#[derive(SystemParam)]
pub struct ThrownArms<'w, 's> {
    /// Stats and magazine of each wielded weapon.
    pub weapons: ThrowWeaponQuery<'w, 's>,
    /// Wielded-weapon link on each combatant.
    pub wields:  Query<'w, 's, &'static Wields>,
    /// Melee weapon filter.
    pub melee:   MeleeQuery<'w, 's>,
}

/// Grids and RNGs used while resolving a throw.
#[derive(SystemParam)]
pub struct ThrowWorld<'w> {
    /// Who stands where.
    pub occupancy:    Res<'w, OccupancyGrid>,
    /// Slab surfaces the arc marches over.
    pub surface:      Res<'w, SurfaceGrid>,
    /// Cover ledger the blast damages.
    pub cover:        ResMut<'w, crate::cover::CoverLedger>,
    /// Slab ledger the blast damages.
    pub slab:         ResMut<'w, SlabLedger>,
    /// Stair cells that grant a brace bonus.
    pub brace_cells:  Res<'w, BraceStairCells>,
    /// Shot roll stream.
    pub shot_rng:     ResMut<'w, ShotRng>,
    /// Wound severity stream.
    pub severity_rng: ResMut<'w, SeverityRng>,
    /// Injury draw stream.
    pub injury_rng:   ResMut<'w, InjuryRng>,
}

pub(super) struct GrenadeStats {
    pub(super) base_spread: BaseSpread,
    pub(super) accuracy:    Accuracy,
    pub(super) kickback:    Kickback,
    pub(super) fatal_bias:  FatalBias,
    pub(super) damage:      WeaponDamage,
    pub(super) punch:       WeaponPunch,
    pub(super) shred:       WeaponShred,
    pub(super) damage_type: DamageType,
    pub(super) stable:      Stable,
    pub(super) brace_bonus: Option<WeaponBraceBonus>,
    pub(super) dot:         Option<DotProfile>,
    pub(super) hit_type:    HitType,
}

impl GrenadeStats {
    pub(super) const fn stats(&self) -> WeaponStats<'_> {
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
