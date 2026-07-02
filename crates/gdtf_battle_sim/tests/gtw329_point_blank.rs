//! GTW-329 POINT-BLANK regression test: a shooter with an enemy in the
//! immediately-adjacent cell along its facing CAN strike that enemy.
//!
//! The bug: a shot at a point-blank (immediately-adjacent) enemy "never connects"
//! when the enemy is at a lower stance (crouching / prone). The march bands the §2
//! clearance test against the round's height only at the voxel ENTRY boundary. For a
//! steep, very-short shot the round's z changes a lot WITHIN one cell: a standing
//! shooter's high muzzle enters the adjacent cell still in the MID/HIGH band, clears
//! the lower-stanced enemy at that entry boundary, and then dives past it into the
//! ground — even though the round physically descends through the enemy's band before
//! it leaves the cell.
//!
//! The fix (`march/vector.rs` + `march/dda.rs`, GTW-329): band the per-voxel
//! clearance test against the LOWEST band the round occupies anywhere INSIDE the
//! voxel (the lower of its entry and exit bands — its height is monotone across a
//! cell), so a point-blank shot that dips into the enemy's band mid-cell connects.
//! Flat shots are unchanged (entry band == exit band).
//!
//! Two layers, both on the REAL code path:
//!  * a low-level `march_vector` sweep over every facing × shooter-stance ×
//!    target-stance point-blank combination (the precise mechanism — RED pre-fix for
//!    every lower-stanced target), and
//!  * the public `fire()` volley for the representative standing-vs-prone case (the
//!    end-to-end seam the act layer drives).
//!
//! Both are DETERMINISTIC: a ZERO base spread collapses the cone to the central axis,
//! so whether the round connects is purely geometric, independent of the RNG stream.
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out
//! (a)); no function here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    ecs::system::SystemState,
    math::Vec3,
    prelude::{Entity, MinimalPlugins, World},
};
use gdtf_battle_sim::{
    Accuracy, Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType,
    BaseSpread, BattleGrids, BattleSeed, BodyPart, BraceStairCells, Cell, CellLevel, CombatTuning,
    CoverLedger, DamageProfile, DamageType, Direction, Facing, FatalBias, FireMode, FireModeSpec,
    Handedness, HandlingProfile, HeightBand, Hp, InflictedWounds, InjuryRegistry, InjuryRng,
    InjuryTables, Kickback, Level, LifeState, Luck, Magazine, MagazineSize, MarchKind, MeleeQuery,
    ModeConeMult, ModeKind, ModeShots, ModeTuPercent, MountedQuery, OccupancyGrid,
    OccupancyMaintenancePlugin, PieceQuery, Position, PriorShots, RecoilClimb, RecoilGrowth,
    ReloadTu, SeverityRng, ShooterQuery, Shooting, ShotKind, ShotRng, Shove, SlabLedger, Stable,
    Stance, StanceKind, SurfaceGrid, TargetQuery, Toughness, Tu, TuMax, Volley, WeaponBundle,
    WeaponDamage, WeaponName, WeaponPunch, WeaponQuery, WeaponShred, WearsQuery, WieldedBy,
    WieldsQuery, WornBy, Wounds, climb_aim_dir, fire::FireOrder, march_vector, muzzle_position,
    target_aim_point,
};

/// The shooter's cell (interior of the grid so every facing has an adjacent cell).
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

/// The cell immediately East of the shooter — the point-blank enemy cell for the
/// representative `fire()` assertion (an East-facing shooter).
fn east_enemy_cell() -> CellLevel {
    CellLevel::new(Cell::new(6, 6), Level::new(0))
}

/// The published silhouette band a ganger in `stance` presents to the march
/// (Standing→HIGH, Crouching→MID, Prone→LOW) — mirrors `OccupancyGrid`'s maintained
/// band so the low-level probe sets the same state the production sync would.
const fn stance_band(stance: StanceKind) -> HeightBand {
    match stance {
        StanceKind::Standing => HeightBand::High,
        StanceKind::Crouching => HeightBand::Mid,
        StanceKind::Prone => HeightBand::Low,
    }
}

/// One single-shot fire-mode spec with a unit cone multiplier (so it does not widen
/// the ZERO base spread — the cone stays ≈ 0 and the trajectory is the central axis).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
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

// --- The low-level march sweep: the precise mechanism. ---

/// March the point-blank muzzle→aim ray for a `shooter_stance` shooter at a
/// `target_stance` enemy one cell away along `facing`, and report whether the round
/// stops on that adjacent occupant. The aim is the enemy's band midpoint (the
/// production aim, GTW-314) and the cone is dead-center (zero recoil), so the result
/// is purely the march geometry.
fn point_blank_march_hits(
    facing: Direction,
    enemy_cell: CellLevel,
    shooter_stance: StanceKind,
    target_stance: StanceKind,
) -> bool {
    let tuning = CombatTuning::default();

    let mut probe_world = World::new();
    let enemy = probe_world.spawn_empty().id();
    let mut occupancy = OccupancyGrid::new();
    occupancy.set_occupant(enemy_cell, Some(enemy));
    occupancy.set_occupant_band(enemy_cell, Some(stance_band(target_stance)));
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    let shooter_pos = Position::new(shooter_cell());
    let muzzle = muzzle_position(
        shooter_pos,
        Facing::new(facing),
        Stance::new(shooter_stance),
        &tuning,
    );
    let aim = target_aim_point(
        Position::new(enemy_cell),
        Stance::new(target_stance),
        Some(stance_band(target_stance)),
        &tuning,
    );
    let dir: Vec3 = climb_aim_dir(
        muzzle,
        aim,
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
    .vec();

    let result = march_vector(
        muzzle,
        dir,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        *shooter_pos,
        |_| false,
    );
    result.kind == MarchKind::Ganger(enemy)
}

/// GTW-329 — a point-blank shot connects for EVERY facing × shooter-stance ×
/// target-stance combination. PRE-FIX the lower-stanced (crouch / prone) targets all
/// MISS (the round clears them at the entry boundary and dives to the ground); this
/// sweep collects and reports every miss, so a regression names the exact case.
#[test]
fn point_blank_march_connects_for_every_facing_and_stance() {
    // Each facing paired with the cell immediately along it from the shooter (5, 6).
    let facings = [
        (Direction::North, Cell::new(5, 5)),
        (Direction::NorthEast, Cell::new(6, 5)),
        (Direction::East, Cell::new(6, 6)),
        (Direction::SouthEast, Cell::new(6, 7)),
        (Direction::South, Cell::new(5, 7)),
        (Direction::SouthWest, Cell::new(4, 7)),
        (Direction::West, Cell::new(4, 6)),
        (Direction::NorthWest, Cell::new(4, 5)),
    ];
    let stances = [
        StanceKind::Standing,
        StanceKind::Crouching,
        StanceKind::Prone,
    ];

    let mut misses: Vec<String> = Vec::new();
    for (facing, enemy_xy) in facings {
        let enemy_cell = CellLevel::new(enemy_xy, Level::new(0));
        for shooter_stance in stances {
            for target_stance in stances {
                if !point_blank_march_hits(facing, enemy_cell, shooter_stance, target_stance) {
                    misses.push(format!(
                        "{facing:?} shooter={shooter_stance:?} target={target_stance:?}"
                    ));
                }
            }
        }
    }

    assert!(
        misses.is_empty(),
        "every point-blank shot must connect; {} missed: {misses:?}",
        misses.len(),
    );
}

// --- The end-to-end fire() seam: the representative standing-vs-prone case. ---

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
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode()]),
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
    let mut rng = ShotRng::from_root(BattleSeed::new(seed));
    let mut sev_rng = SeverityRng::from_root(BattleSeed::new(seed));
    let mut injury_rng = InjuryRng::from_root(BattleSeed::new(seed));
    let mode = single_mode();

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
            gdtf_battle_sim::fire(
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
