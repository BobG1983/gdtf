//! Destroyed-cover bridge: fire frees the cell and opens LOS.

use bevy::{
    app::App,
    ecs::message::Messages,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    march::{MarchDir, MarchGrids, MarchKind, march_vector},
    occupancy::TerrainKind,
    occupancy_sync::{OccupancyMaintenancePlugin, TerrainPieceDestroyed},
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

const fn low_hp_piece(kind: TerrainPieceKind) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
        kind,
    )
}

const fn low_hp_cover() -> CoverEntry {
    low_hp_piece(TerrainPieceKind::Cover)
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
        MarchGrids {
            occupancy,
            surface,
            cover,
        },
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
        assert_eq!(
            grid.terrain(&cover_cell()),
            TerrainKind::Cover,
            "BEFORE: the cover piece must still be standing in the cell",
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
    assert_eq!(
        grid.terrain(&cover_cell()),
        TerrainKind::Open,
        "AFTER: replace_destroyed_piece must have cleared the smashed cell, because this app \
         holds no def registry and so nothing is left behind (the bridge drove the real wiring)",
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

fn fire_and_drain(app: &mut App, shooter: Entity, at: CellLevel) -> Vec<TerrainPieceDestroyed> {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(at.x, at.y),
        Level::new(0),
    ));
    app.update();
    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .drain()
        .collect()
}

#[test]
fn a_fired_round_reports_the_kind_of_the_piece_it_destroyed() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());
    app.insert_resource(OccupancyGrid::new());

    let wall_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    let cover_at = CellLevel::new(Cell::new(10, 5), Level::new(0));
    let mut cover = CoverLedger::new();
    cover.insert(wall_at, low_hp_piece(TerrainPieceKind::Wall));
    cover.insert(cover_at, low_hp_piece(TerrainPieceKind::Cover));
    app.insert_resource(cover);

    // The wall sits between the shooter and the cover, so it falls first.
    let mut kinds: Vec<TerrainPieceKind> = Vec::new();
    for _ in 0..8 {
        if kinds.len() >= 2 {
            break;
        }
        kinds.extend(
            fire_and_drain(&mut app, shooter, cover_at)
                .into_iter()
                .map(|message| message.kind),
        );
    }

    assert_eq!(
        kinds,
        vec![TerrainPieceKind::Wall, TerrainPieceKind::Cover],
        "a destroyed piece reports the kind its ledger entry held — the wall first, then the \
         cover behind it, found {kinds:?}",
    );
}
