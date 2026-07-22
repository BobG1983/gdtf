//! The end-to-end `fire()` path: a full-app point-blank volley strikes the
//! adjacent PRONE enemy (the representative standing-vs-prone case).

use bevy::{
    app::App,
    ecs::system::SystemState,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverLedger, HeightBand},
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

use super::harness::*;

/// The cell immediately East of the shooter — the point-blank enemy cell for the
/// representative `fire()` assertion (an East-facing shooter).
fn east_enemy_cell() -> CellLevel {
    CellLevel::new(Cell::new(6, 6), Level::new(0))
}

/// Equip a ganger's six worn-armor-piece entities (GTW-323 / ADR-0004) at the thin
/// uniform stats, related via `WornBy` so `fire()` resolves the struck location through
/// `ganger → Wears → the BodyPart-tagged piece` (the relationship hook populates `Wears`
/// synchronously in a bare `World` spawn).
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

// --- The end-to-end fire() path: the representative standing-vs-prone case. ---

/// Build the real-path app: `MinimalPlugins` + `OccupancyMaintenancePlugin` (whose
/// `sync_moved_gangers` publishes each occupant's stance-derived silhouette band off
/// the grid), plus the sim resources `fire()` reads.
fn point_blank_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

/// Spawn an armed, alive, loaded, aiming, STANDING shooter at the shooter cell facing
/// East. ZERO base spread, so the trajectory is the muzzle→aim central axis EXACTLY.
fn spawn_standing_shooter(app: &mut App) -> Entity {
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
            Facing::new(Direction::East),
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
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`); the
    // `WieldedBy` insert hook populates the ganger's `Wields` synchronously in a bare
    // `World` spawn so the very next `fire()` resolves it.
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// Spawn a PRONE enemy at the point-blank cell immediately East (the production
/// `sync_moved_gangers` publishes its LOW silhouette band).
fn spawn_prone_enemy(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(east_enemy_cell()),
            Stance::new(StanceKind::Prone),
            Hp::new(30),
            Wounds::new(6),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id()
}

/// Fire ONE volley at the enemy cell with seed `seed`, returning the resolved
/// [`Volley`].
fn fire_one_volley(app: &mut App, shooter: Entity, seed: u64) -> Volley {
    /// The `fire()` query tuple, aliased so the `SystemState` type stays under clippy's
    /// `type_complexity` gate (the GTW-543 mounted-weapon `MountedQuery` addition tipped it over).
    /// Declared FIRST in the fn so it precedes the `let`s (`items_after_statements`).
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
                    target_cell:  Cell::new(east_enemy_cell().x, east_enemy_cell().y),
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

/// The struck [`ShotKind`] of the volley's single round, or `None` if it fired no
/// rounds (keeps the test panic-free).
fn single_shot_kind(volley: &Volley) -> Option<ShotKind> {
    volley.shots.first().map(|outcome| outcome.kind)
}

/// GTW-329 end-to-end — a standing shooter at point-blank STRIKES the prone enemy in
/// the immediately-adjacent cell via the public `fire()` volley. The prone enemy's
/// published band is LOW; the round dips into the LOW band inside the adjacent cell,
/// so it must impact (`ShotKind::Ganger`). PRE-FIX the round cleared the prone enemy
/// at the entry boundary (still MID/HIGH there) and dove into the ground → MISS.
#[test]
fn point_blank_fire_strikes_the_adjacent_prone_enemy() {
    let mut app = point_blank_app();
    let shooter = spawn_standing_shooter(&mut app);
    let enemy = spawn_prone_enemy(&mut app);
    // GTW-323: equip each ganger's worn-armor PIECE entities (the `fire()` armor path).
    equip_thin_armor(&mut app, shooter);
    equip_thin_armor(&mut app, enemy);
    // ONE update: `sync_moved_gangers` publishes each occupant's band off the grid.
    app.update();

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&east_enemy_cell()));
    assert_eq!(
        band,
        Some(HeightBand::Low),
        "a prone enemy presents the LOW silhouette band; got {band:?}",
    );

    let volley = fire_one_volley(&mut app, shooter, 0xC0FF_EE29);
    assert_eq!(
        single_shot_kind(&volley),
        Some(ShotKind::Ganger(enemy)),
        "the point-blank shot must IMPACT the immediately-adjacent prone enemy, not \
         clear it at the entry boundary and dive into the ground",
    );
}
