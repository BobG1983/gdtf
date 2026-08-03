//! Query and SystemParam bundles for melee dispatch.

use bevy::{
    ecs::system::SystemParam,
    prelude::{MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::{InjuryInflicted, request::MeleeStruck},
    armor_wear::ArmorBroken,
    cover::CoverLedger,
    ganger::{Facing, Faction, Fight, Hp, LifeState, Luck, Position, Stance, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InflictedInjuries, InjuryRegistry, InjuryTables},
    occupancy::OccupancyGrid,
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, FightMode, Shove, WeaponDamage, WeaponPunch, WeaponShred},
};

pub(super) type MeleeGeomQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static Stance,
        &'static Facing,
        &'static Fight,
        &'static Faction,
        &'static Luck,
    ),
>;

pub(super) type MeleeTargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        Option<&'static InflictedInjuries>,
    ),
>;

pub(super) type MeleeWeaponQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static WeaponDamage,
        &'static WeaponPunch,
        &'static WeaponShred,
        &'static DamageType,
        &'static FatalBias,
        &'static FightMode,
        &'static Shove,
    ),
>;

/// Grids, tuning, and injury content for melee.
#[derive(SystemParam)]
pub struct MeleeWorld<'w> {
    pub(super) occupancy: Res<'w, OccupancyGrid>,
    pub(super) surface: Res<'w, SurfaceGrid>,
    pub(super) cover: ResMut<'w, CoverLedger>,
    pub(super) tuning: Res<'w, CombatTuning>,
    pub(super) tables: Option<Res<'w, InjuryTables>>,
    pub(super) registry: Option<Res<'w, InjuryRegistry>>,
}

/// Optional RNGs required for a melee contest.
#[derive(SystemParam)]
pub struct MeleeRngs<'w> {
    pub(super) fight: Option<ResMut<'w, FightRng>>,
    pub(super) shot: Option<ResMut<'w, ShotRng>>,
    pub(super) severity: Option<ResMut<'w, SeverityRng>>,
    pub(super) injury: Option<ResMut<'w, InjuryRng>>,
}

/// Writers for struck / armor break / injury outcomes.
#[derive(SystemParam)]
pub struct MeleeFacts<'w> {
    pub(super) struck: MessageWriter<'w, MeleeStruck>,
    pub(super) breaks: MessageWriter<'w, ArmorBroken>,
    pub(super) injuries: MessageWriter<'w, InjuryInflicted>,
}
