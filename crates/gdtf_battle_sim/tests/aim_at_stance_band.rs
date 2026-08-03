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
        Volley, WeaponQuery, WearsQuery, WieldsQuery,
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
    resolve_coarse::ShotKind,
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

fn target_cell() -> CellLevel {
    CellLevel::new(Cell::new(12, 6), Level::new(0))
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

fn aim_band_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

fn spawn_standing_shooter(app: &mut App, facing: Direction) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        BaseSpread::new(0.0),
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

fn spawn_target(app: &mut App, stance: StanceKind) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(target_cell()),
            Stance::new(stance),
            Hp::new(30),
            Wounds::new(6),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id()
}

fn fire_one_volley(app: &mut App, shooter: Entity, seed: u64) -> Volley {
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
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(seed);
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
    let access = state.get_mut(app.world_mut());
    assert!(access.is_ok(), "shooter/target queries must validate");
    let volley = match access {
        Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) => {
            gdtf_battle_sim::fire::fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(target_cell().x, target_cell().y),
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
                &mut rng,
                &mut sev_rng,
                &InjuryTables::default(),
                &InjuryRegistry::default(),
                &mut injury_rng,
            )
        }
        Err(_) => Volley {
            reports: Vec::new(),
            shots:   Vec::new(),
            splash:  Vec::new(),
        },
    };
    state.apply(app.world_mut());
    volley
}

fn single_shot_kind(volley: &Volley) -> Option<ShotKind> {
    volley.shots.first().map(|outcome| outcome.kind)
}

fn scene_with_target(stance: StanceKind) -> (App, Entity, Entity) {
    let mut app = aim_band_app();
    let facing = Direction::from_cells(
        Cell::new(shooter_cell().x, shooter_cell().y),
        Cell::new(target_cell().x, target_cell().y),
    )
    .unwrap_or(Direction::East);
    let shooter = spawn_standing_shooter(&mut app, facing);
    let target = spawn_target(&mut app, stance);
    equip_thin_armor(&mut app, shooter);
    equip_thin_armor(&mut app, target);
    app.update();
    (app, shooter, target)
}

#[test]
fn standing_shooter_hits_a_crouching_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Crouching);

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::cover::HeightBand::Mid),
        "a crouching ganger presents the MID silhouette band; got {band:?}",
    );

    let volley = fire_one_volley(&mut app, shooter, 0xC0FF_EE01);
    assert_eq!(
        single_shot_kind(&volley),
        Some(ShotKind::Ganger(target)),
        "the standing shooter's central axis must IMPACT the crouching target (the aim \
         must land in the MID band the clearance test checks, not sail over it at HIGH)",
    );
}

#[test]
fn standing_shooter_hits_a_prone_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Prone);

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::cover::HeightBand::Low),
        "a prone ganger presents the LOW silhouette band; got {band:?}",
    );

    let volley = fire_one_volley(&mut app, shooter, 0xC0FF_EE02);
    assert_eq!(
        single_shot_kind(&volley),
        Some(ShotKind::Ganger(target)),
        "the standing shooter's central axis must IMPACT the prone target (the aim must \
         land in the LOW band the clearance test checks, not sail over it at HIGH)",
    );
}

#[test]
fn standing_shooter_still_hits_a_standing_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Standing);

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::cover::HeightBand::High),
        "a standing ganger presents the HIGH silhouette band; got {band:?}",
    );

    let volley = fire_one_volley(&mut app, shooter, 0xC0FF_EE03);
    assert_eq!(
        single_shot_kind(&volley),
        Some(ShotKind::Ganger(target)),
        "the standing-vs-standing shot must still IMPACT (the HIGH band-midpoint aim \
         stays in the HIGH band, so the clearance verdict is unchanged from pre-fix)",
    );
}
