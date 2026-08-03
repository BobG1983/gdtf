use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    march::{MarchDir, MarchKind, march_vector},
    occupancy::TerrainKind,
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, SimPos,
        Stance, StanceKind, Tu,
    },
    surface::SurfaceGrid,
    test_support::{SimAppBuilder, single_mode},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), Level::new(0))
}

fn cover_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), Level::new(0))
}

fn bridge_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(0xC0BA_17C0)
        .with_acts()
        .with_player_faction(1)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("breacher".to_owned()),
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(200),
            WeaponPunch::new(80),
            WeaponShred::new(40),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![single_mode(0.2, 1)]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = world
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(100),
            Faction::new(1),
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

const fn low_hp_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

fn probe_stops_on_cover(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    let muzzle = SimPos::new(2.5, 5.5, 0.9);
    let dir = bevy::math::Vec3::new(1.0, 0.0, 0.0);
    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        occupancy,
        surface,
        cover,
        tuning,
        shooter_cell(),
        |_| false,
    );
    matches!(result.kind, MarchKind::Cover(_))
}

#[test]
fn fired_round_destroys_cover_and_the_bridge_frees_the_cell() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    let mut occupancy = OccupancyGrid::new();
    occupancy.set_terrain(cover_cell(), TerrainKind::Cover);
    app.insert_resource(occupancy);
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell(), low_hp_cover());
    app.insert_resource(cover);

    let tuning = CombatTuning::default();
    let surface_before = SurfaceGrid::new();
    {
        let grid = app.world().resource::<OccupancyGrid>();
        let cover_res = app.world().resource::<CoverLedger>();
        assert!(
            *grid.is_blocked(&cover_cell()),
            "BEFORE: the standing cover cell must block",
        );
        assert!(
            !*grid.is_cover_destroyed(&cover_cell()),
            "BEFORE: the cover cell must not yet be in the destroyed-cover set",
        );
        assert!(
            probe_stops_on_cover(grid, &surface_before, cover_res, &tuning),
            "BEFORE: a probe LOS/path must STOP on the standing cover",
        );
    }

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(cover_cell().x, cover_cell().y),
        Level::new(0),
    ));
    app.update();
    app.update();

    let grid = app.world().resource::<OccupancyGrid>();
    let cover_res = app.world().resource::<CoverLedger>();
    assert!(
        *grid.is_cover_destroyed(&cover_cell()),
        "AFTER: sync_destroyed_cover must have marked the smashed cell destroyed (the bridge \
         drove the real wiring) — got destroyed-set miss",
    );
    assert!(
        !*grid.is_blocked(&cover_cell()),
        "AFTER: a destroyed cover cell must stop blocking (the freed cell)",
    );
    assert!(
        !probe_stops_on_cover(grid, &surface_before, cover_res, &tuning),
        "AFTER: a probe LOS/path must now PASS through the freed cell (a previously-blocked \
         sightline opened)",
    );
}
