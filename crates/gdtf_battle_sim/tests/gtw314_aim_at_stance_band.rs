//! GTW-314 AIM-AT-STANCE-BAND regression test: a standing shooter HITS a crouching /
//! prone target on the central axis instead of sailing over it.
//!
//! The bug: `fire/compose.rs::TargetGeometry::compose` hardcoded the target's aim
//! stance to `StanceKind::Standing` regardless of the target's REAL stance. That fed
//! `ShotInputs::target_stance` → `target_aim_point`, which picked the standing
//! silhouette-top (~0.95 of a level), so a standing muzzle's central axis arrived at
//! the target cell in the HIGH band. The §2 clearance test
//! (`round_clears_occupant`) then saw a HIGH round vs a MID (crouch) / LOW (prone)
//! occupant — strictly higher → the round CLEARS (sails over) → a systematic miss
//! against any non-standing target. (GTW-306 fixed only the presenter sprite; the
//! sim trajectory was never fixed.)
//!
//! The fix routes the target's PUBLISHED silhouette band (GTW-304's
//! `OccupancyGrid::occupant_band`) through `target_aim_point`'s band-midpoint branch,
//! so the central axis lands squarely INSIDE the band the clearance test compares
//! against (Crouch → MID midpoint, Prone → LOW midpoint).
//!
//! This test exercises the REAL fix path — the public `fire()` volley, which calls
//! `TargetGeometry::compose` internally — never reaching for the crate-private
//! geometry. It is DETERMINISTIC: the shooter's weapon has a ZERO base spread, so the
//! composed cone is ≈ 0 and the trajectory is dead-center on the muzzle→aim central
//! axis regardless of the RNG stream; whether that axis impacts the occupant
//! (`ShotKind::Ganger`) or sails over it (`ShotKind::Miss`) is then purely geometric.
//!
//! On the PRE-FIX code the aim is forced Standing (HIGH), so the central axis sails
//! over a crouching / prone occupant → `ShotKind::Miss` → these tests FAIL. After the
//! fix the aim lands in the occupant's own band → `ShotKind::Ganger` → they pass. A
//! standing-vs-standing control proves the fix does not regress the already-working
//! case. Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7
//! carve-out (a)); no function here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    ecs::system::SystemState,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    Accuracy, Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType,
    BaseSpread, BattleGrids, BattleSeed, BodyPart, BraceStairCells, Cell, CellLevel, CombatTuning,
    CoverLedger, DamageProfile, DamageType, Direction, Facing, FatalBias, FireMode, FireModeSpec,
    HandlingProfile, Hp, InflictedWounds, Kickback, Level, LifeState, Luck, Magazine, MagazineSize,
    ModeConeMult, ModeKind, ModeShots, ModeTuPercent, OccupancyGrid, OccupancyMaintenancePlugin,
    PieceQuery, Position, ReloadTu, SeverityRng, ShooterQuery, Shooting, ShotKind, ShotRng,
    SlabLedger, Stable, Stance, StanceKind, SurfaceGrid, TargetQuery, Toughness, Tu, TuMax, Volley,
    WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponQuery, WeaponShred, WearsQuery,
    WieldedBy, WieldsQuery, WornBy, Wounds, fire::FireOrder,
};

/// The shooter's cell.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

/// The target's cell — a flat East shot from the shooter, same storey.
fn target_cell() -> CellLevel {
    CellLevel::new(Cell::new(12, 6), Level::new(0))
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

/// Build the real-path app: `MinimalPlugins` + `OccupancyMaintenancePlugin` (whose
/// `sync_moved_gangers` publishes each occupant's stance-derived silhouette band off
/// the grid), plus the sim resources `fire()` reads. NO `set_occupant_band` call —
/// the band that drives the aim is published by REAL code.
fn aim_band_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

/// Spawn an armed, alive, loaded, aiming, STANDING shooter at the shooter cell facing
/// the target. The weapon's `BaseSpread` is ZERO, so the composed cone is ≈ 0 and the
/// sampled trajectory is the muzzle→aim central axis EXACTLY — the geometric HIT/MISS
/// is then deterministic regardless of the seed.
fn spawn_standing_shooter(app: &mut App, facing: Direction) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        // ZERO spread: the cone collapses to the central axis (cone 0 = the axis).
        BaseSpread::new(0.0),
        Accuracy::new(4.0),
        // ZERO kickback so there is no recoil widening across the (single) round.
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
            // `stable` so the brace engages unconditionally — keeps the cone tight,
            // though the ZERO base spread already collapses it to the axis.
            Stable::new(true),
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
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`); the
    // `WieldedBy` insert hook populates the ganger's `Wields` synchronously in a bare
    // `World` spawn so the very next `fire()` resolves it.
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// Spawn a target ganger at the target cell holding `stance` (so the production
/// `sync_moved_gangers` publishes its silhouette band — Standing→HIGH, Crouching→MID,
/// Prone→LOW), carrying the full `TargetQuery` battle-surface set.
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

/// Fire ONE volley at the target cell with seed `seed`, returning the resolved
/// [`Volley`]. Reads the maintained grids straight off the world (cloned read views so
/// the two disjoint fire queries can borrow the world mutably without aliasing the
/// resources).
fn fire_one_volley(app: &mut App, shooter: Entity, seed: u64) -> Volley {
    let tuning = CombatTuning::default();
    let mut rng = ShotRng::from_root(BattleSeed::new(seed));
    let mut sev_rng = SeverityRng::from_root(BattleSeed::new(seed));
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

    let mut state: SystemState<(
        ShooterQuery,
        TargetQuery,
        WearsQuery,
        PieceQuery,
        WieldsQuery,
        WeaponQuery,
    )> = SystemState::new(app.world_mut());
    // `get_mut` now returns a `Result` (Bevy 0.19); the params always validate, so
    // an `Err` is structurally impossible — assert loudly rather than firing nothing.
    let access = state.get_mut(app.world_mut());
    assert!(access.is_ok(), "shooter/target queries must validate");
    let volley = match access {
        Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons)) => {
            gdtf_battle_sim::fire(
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
            )
        }
        Err(_) => Volley {
            reports: Vec::new(),
            shots:   Vec::new(),
        },
    };
    state.apply(app.world_mut());
    volley
}

/// The struck [`ShotKind`] of the volley's single round, or `None` if it fired no
/// rounds (keeps the test panic-free — all `unwrap`/`expect`/`panic` are denied here).
fn single_shot_kind(volley: &Volley) -> Option<ShotKind> {
    volley.shots.first().map(|outcome| outcome.kind)
}

/// Stand the scene up: a standing shooter facing the `stance` target, one `update()`
/// so the production occupancy-maintenance chain publishes the target's band off the
/// grid, then return `(app, shooter, target)`. The aim band is published by REAL code.
fn scene_with_target(stance: StanceKind) -> (App, Entity, Entity) {
    let mut app = aim_band_app();
    let facing = Direction::from_cells(
        Cell::new(shooter_cell().x, shooter_cell().y),
        Cell::new(target_cell().x, target_cell().y),
    )
    .unwrap_or(Direction::East);
    let shooter = spawn_standing_shooter(&mut app, facing);
    let target = spawn_target(&mut app, stance);
    // GTW-323: equip each ganger's worn-armor PIECE entities (the `fire()` armor path).
    equip_thin_armor(&mut app, shooter);
    equip_thin_armor(&mut app, target);
    // ONE update: `sync_moved_gangers` sees both fresh Positions as `Changed` and
    // publishes each occupant's stance-derived silhouette band off the grid.
    app.update();
    (app, shooter, target)
}

/// GTW-314 — a standing shooter HITS a CROUCHING target on the central axis. The
/// crouching target's published band is MID; the fix aims the central axis at the MID
/// midpoint, so the round impacts it (`ShotKind::Ganger`). PRE-FIX the aim was forced
/// Standing (HIGH) and the round sailed over → `ShotKind::Miss` → this test FAILED.
#[test]
fn standing_shooter_hits_a_crouching_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Crouching);

    // Sanity: the production sync published the crouching target's MID band.
    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::HeightBand::Mid),
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

/// GTW-314 — a standing shooter HITS a PRONE target on the central axis. The prone
/// target's published band is LOW; the fix aims the central axis at the LOW midpoint,
/// so the round impacts it. PRE-FIX the aim was forced Standing (HIGH) → the round
/// sailed over → `ShotKind::Miss` → this test FAILED.
#[test]
fn standing_shooter_hits_a_prone_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Prone);

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::HeightBand::Low),
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

/// GTW-314 control — a standing shooter still HITS a STANDING target on the central
/// axis (the already-working case is NOT regressed). The standing target's published
/// band is HIGH; the fix aims the central axis at the HIGH midpoint (vs the old
/// silhouette-top ~0.95), which still lands in the HIGH band, so the clearance verdict
/// is unchanged — the round impacts.
#[test]
fn standing_shooter_still_hits_a_standing_target_on_the_central_axis() {
    let (mut app, shooter, target) = scene_with_target(StanceKind::Standing);

    let band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&target_cell()));
    assert_eq!(
        band,
        Some(gdtf_battle_sim::HeightBand::High),
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
