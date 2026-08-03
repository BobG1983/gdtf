pub(super) use bevy::prelude::{App, Entity, Messages, Update, World};

pub(super) use crate::test_support::{SimAppBuilder, TEST_PLAYER_GANG, single_mode, target_bundle};
pub(super) use crate::{
    acts::*,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    effects::bleed::BleedingOut,
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Suppressed, SuppressorCell, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    resolve_and_apply::HitVerdict,
    resolve_coarse::ShotKind,
    shot_fired::ShotFired,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy, Wields,
    },
};

pub(super) fn headless_app() -> App {
    SimAppBuilder::new().with_acts().with_full_vision().build()
}

pub(super) fn spawn_shooter(
    world: &mut World,
    x: i32,
    y: i32,
    mode: FireModeSpec,
    aiming: bool,
) -> Entity {
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(LoadedRounds::new(10), mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(aiming),
            Shooting::new(1.0),
            Tu::new(200),
            crate::ganger::TuMax::new(100),
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    world.spawn((WieldedBy::new(shooter), bundle));
    shooter
}

pub(super) fn fire_scenario() -> (App, Entity, Entity) {
    let mut app = headless_app();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
    let target = app.world_mut().spawn(target_bundle(30, 6)).id();
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(target_at, Some(target));
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }
    (app, shooter, target)
}

pub(super) fn spawn_downed_actor(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            LifeState::Alive,
            Faction::new(faction),
        ))
        .id()
}

pub(super) fn spawn_downed_target(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            LifeState::Downed,
            Faction::new(faction),
            BleedingOut,
        ))
        .id()
}
