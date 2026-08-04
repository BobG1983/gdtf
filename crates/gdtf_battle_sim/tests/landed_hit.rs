//! Real-path ganger shot landing through the occupant band publisher.

use bevy::{
    app::App,
    ecs::system::SystemState,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::CoverLedger,
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery,
        WeaponQuery, WearsQuery, WieldsQuery,
    },
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Level, LifeState, OccupancyGrid, Position, Stance, StanceKind,
        Tu,
    },
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    test_support::{injury_rng, severity_rng, shot_rng, single_mode},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

fn enemy_cell() -> CellLevel {
    CellLevel::new(Cell::new(12, 9), Level::new(0))
}

fn equip_thin_armor(app: &mut App, ganger: Entity) {
    for part in BodyPart::ALL {
        app.world_mut().spawn((
            WornBy::new(ganger),
            part,
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(1),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        ));
    }
}

fn landed_hit_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

fn spawn_shooter(app: &mut App, facing: Direction) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        BaseSpread::new(0.02),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
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
    let shooter = app
        .world_mut()
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(200),
            TuMax::new(100),
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
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

fn spawn_enemy(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(enemy_cell()),
            Stance::new(StanceKind::Standing),
            Hp::new(30),
            Wounds::new(6),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id()
}

fn one_volley_lands(app: &mut App, shooter: Entity, enemy: Entity, seed: u64) -> bool {
    type FireQueries<'w, 's> = (
        ShooterQuery<'w, 's>,
        TargetQuery<'w, 's>,
        WearsQuery<'w, 's>,
        PieceQuery<'w, 's>,
        WieldsQuery<'w, 's>,
        WeaponQuery<'w, 's>,
        MeleeQuery<'w, 's>,
        MountedQuery<'w, 's>,
    );
    let hp_before = app.world().get::<Hp>(enemy).map_or(0, |h| **h);
    let wounds_before = app
        .world()
        .get::<InflictedWounds>(enemy)
        .map_or(0, |w| w.len());

    let tuning = CombatTuning::default();
    let mut shot_rng = shot_rng(seed);
    let mut sev_rng = severity_rng(seed);
    let mut injury_rng = injury_rng(seed);
    let mode = single_mode(0.2, 1);

    let occupancy = app
        .world()
        .get_resource::<OccupancyGrid>()
        .cloned()
        .unwrap_or_default();
    let surface = app
        .world()
        .get_resource::<SurfaceGrid>()
        .cloned()
        .unwrap_or_default();
    let mut cover = app
        .world()
        .get_resource::<CoverLedger>()
        .cloned()
        .unwrap_or_default();
    let mut slab = app
        .world()
        .get_resource::<SlabLedger>()
        .cloned()
        .unwrap_or_default();

    let mut state: SystemState<FireQueries> = SystemState::new(app.world_mut());
    {
        let world = app.world_mut();
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) =
            state.get_mut(world)
        else {
            return false;
        };
        let _volley = gdtf_battle_sim::fire::fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(enemy_cell().x, enemy_cell().y),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut targets,
            &wears,
            &mut pieces,
            &wields,
            &mut weapons,
            &melee,
            &mounted,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &tuning,
            &mut shot_rng,
            &mut sev_rng,
            &InjuryTables::default(),
            &InjuryRegistry::default(),
            &mut injury_rng,
        );
    }
    state.apply(app.world_mut());

    let hp_after = app.world().get::<Hp>(enemy).map_or(hp_before, |h| **h);
    let wounds_after = app
        .world()
        .get::<InflictedWounds>(enemy)
        .map_or(wounds_before, |w| w.len());

    hp_after < hp_before || wounds_after > wounds_before
}

#[test]
fn real_path_ganger_shot_lands_on_at_least_one_seed() {
    let mut app = landed_hit_app();

    let facing = Direction::from_cells(
        Cell::new(shooter_cell().x, shooter_cell().y),
        Cell::new(enemy_cell().x, enemy_cell().y),
    )
    .unwrap_or(Direction::East);
    let shooter = spawn_shooter(&mut app, facing);
    let enemy = spawn_enemy(&mut app);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");
    equip_thin_armor(&mut app, shooter);
    equip_thin_armor(&mut app, enemy);

    app.update();

    let enemy_band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&enemy_cell()));
    assert!(
        enemy_band.is_some(),
        "the production sync must publish the enemy's occupant band; got {enemy_band:?}",
    );

    let seeds: [u64; 8] = [
        0x5A1C_AC75,
        0x0BAD_F00D,
        0xDEAD_BEEF,
        0xFEED_FACE,
        0x1234_5678,
        0xCAFE_B0BA,
        0x9E37_79B9,
        0xA11C_E5ED,
    ];
    let mut lands = 0_u32;
    for seed in seeds {
        if one_volley_lands(&mut app, shooter, enemy, seed) {
            lands += 1;
        }
    }

    assert!(
        lands > 0,
        "at least one of {} seeds must LAND a shot on the enemy (Hp drop / wound) — got {lands} \
         hits. 0 hits means the occupant band is never published and the round passes straight \
         through the ganger.",
        seeds.len(),
    );
}
