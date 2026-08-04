//! ECS query types used by the fire path.

use bevy::{
    ecs::{
        query::With,
        system::{Query, SystemParam},
    },
    prelude::Entity,
};

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

/// Shooter components needed to fire.
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

/// Wielded weapon link on a combatant.
pub type WieldsQuery<'world, 'state> = Query<'world, 'state, &'static Wields>;

/// Filter for melee weapons.
pub type MeleeQuery<'world, 'state> = Query<'world, 'state, (), With<MeleeWeapon>>;

/// Filter for mounted weapons.
pub type MountedQuery<'world, 'state> = Query<'world, 'state, (), With<MountedWeapon>>;

/// Weapon stats and magazine for a wielded weapon.
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

/// Target combatant mutable state for damage apply.
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

/// Worn armor set on a combatant.
pub type WearsQuery<'world, 'state> = Query<'world, 'state, &'static Wears>;

/// Individual armor piece data.
pub type PieceQuery<'world, 'state> = Query<'world, 'state, PieceArmorMut, With<WornBy>>;

/// The weapon a combatant wields, with the melee and mounted probes on it.
#[derive(SystemParam)]
pub struct WieldedWeapons<'w, 's> {
    /// Wielded-weapon link on each combatant.
    pub wields:  WieldsQuery<'w, 's>,
    /// Weapon stats and magazine.
    pub weapons: WeaponQuery<'w, 's>,
    /// Melee weapon filter.
    pub melee:   MeleeQuery<'w, 's>,
    /// Mounted weapon filter.
    pub mounted: MountedQuery<'w, 's>,
}

/// The bodies and worn armor of everyone a shot can strike.
#[derive(SystemParam)]
pub struct StruckBodies<'w, 's> {
    /// Mutable combatant state.
    pub targets: TargetQuery<'w, 's>,
    /// Worn armor sets.
    pub wears:   WearsQuery<'w, 's>,
    /// Individual armor pieces.
    pub pieces:  PieceQuery<'w, 's>,
}

/// Shared battle grids passed into fire resolution.
#[derive(Debug)]
pub struct BattleGrids<'a> {
    /// Occupancy grid.
    pub occupancy:   &'a OccupancyGrid,
    /// Surface / slab grid.
    pub surface:     &'a SurfaceGrid,
    /// Cover ledger (mutable for damage).
    pub cover:       &'a mut CoverLedger,
    /// Slab ledger (mutable for damage).
    pub slab:        &'a mut SlabLedger,
    /// Brace-capable stair cells.
    pub brace_cells: &'a BraceStairCells,
}

/// One fire order: who fires, in what mode, at which cell/level.
#[derive(Debug, Clone, Copy)]
pub struct FireOrder<'a> {
    /// Combatant pulling the trigger.
    pub shooter:      Entity,
    /// Fire mode being used.
    pub mode:         &'a FireModeSpec,
    /// Target cell.
    pub target_cell:  Cell,
    /// Target level.
    pub target_level: Level,
}
