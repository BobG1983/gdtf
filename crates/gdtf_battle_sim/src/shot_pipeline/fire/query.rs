use bevy::ecs::{query::With, system::Query};

use crate::{
    armor::{PieceArmorMut, Wears, WornBy},
    cover::CoverLedger,
    effects::attachments::WeaponBraceBonus,
    ganger::{
        Aiming, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, Suppressed, Toughness, Tu,
        TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    weapon::{
        Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireModeSpec, Handedness,
        Kickback, MeleeWeapon, MountedWeapon, Stable, WeaponDamage, WeaponPunch, WeaponShred,
        WieldedBy, Wields,
    },
};

pub type ShooterQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        (
            &'static Position,
            &'static Facing,
            &'static Stance,
            &'static Aiming,
            &'static Shooting,
            &'static Luck,
            &'static TuMax,
            Option<&'static Suppressed>,
        ),
        Option<&'static InflictedInjuries>,
        &'static mut Tu,
    ),
>;

pub type WieldsQuery<'world, 'state> = Query<'world, 'state, &'static Wields>;

pub type MeleeQuery<'world, 'state> = Query<'world, 'state, (), With<MeleeWeapon>>;

pub type MountedQuery<'world, 'state> = Query<'world, 'state, (), With<MountedWeapon>>;

pub type WeaponQuery<'world, 'state> = Query<
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
        &'static Handedness,
        &'static mut Magazine,
        Option<&'static DotProfile>,
    ),
    With<WieldedBy>,
>;

pub type TargetQuery<'world, 'state> = Query<
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

pub type WearsQuery<'world, 'state> = Query<'world, 'state, &'static Wears>;

pub type PieceQuery<'world, 'state> = Query<'world, 'state, PieceArmorMut, With<WornBy>>;

#[derive(Debug)]
pub struct BattleGrids<'a> {
        pub occupancy:   &'a OccupancyGrid,
        pub surface:     &'a SurfaceGrid,
                pub cover:       &'a mut CoverLedger,
                    pub slab:        &'a mut SlabLedger,
                pub brace_cells: &'a BraceStairCells,
}

#[derive(Debug, Clone, Copy)]
pub struct FireOrder<'a> {
        pub mode:         &'a FireModeSpec,
        pub target_cell:  Cell,
        pub target_level: Level,
}
