//! GTW-304 LANDED-HIT regression test: a ganger shot can actually LAND.
//!
//! The bug: `march/dda.rs::impact_at` strikes a ganger only when BOTH the cell's
//! `occupant` AND its `occupant_band` are `Some`. The change-driven maintenance
//! (`occupancy_sync::sync_moved_gangers`) published the OCCUPANT but never the BAND,
//! so every real ganger's `occupant_band` stayed `None`, rounds passed straight
//! through, and 0 / N shots landed. (The GTW-289 end-to-end test masked this by
//! calling `set_occupant_band` BY HAND.)
//!
//! This test exercises the REAL band-publishing path — it never calls
//! `set_occupant_band`. It spawns a player shooter and an enemy ganger (each with a
//! real `Position` + `Stance`) into the live `OccupancyMaintenancePlugin` chain,
//! runs one `update()` so the production `sync_moved_gangers` publishes each
//! occupant's stance-derived silhouette band off the grid, then runs the real
//! `fire()` volley across several seeds and asserts a shot LANDS on the enemy on at
//! least one seed: the enemy's `Hp` drops OR an `InflictedWound` is recorded.
//!
//! On the PRE-FIX code the enemy's band is never published, so `impact_at` returns
//! `None` for the ganger on every round of every seed → 0 hits → this test FAILS.
//! After the fix the band is published by `sync_moved_gangers` and hits land.
//!
//! Seed-robust: it asserts "hits occur on >= 1 of N seeds", never a pinned damage
//! magnitude. Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md`
//! #7 carve-out (a)); no function here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    ecs::system::SystemState,
    prelude::{Entity, MinimalPlugins},
};
use gdtf_battle_sim::{
    Accuracy, Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType,
    BaseSpread, BattleGrids, BattleSeed, BodyPart, BraceStairCells, Cell, CellLevel, CombatTuning,
    CoverLedger, DamageProfile, DamageType, Direction, Facing, FatalBias, FireMode, FireModeSpec,
    Handedness, HandlingProfile, Hp, InflictedWounds, InjuryRegistry, InjuryRng, InjuryTables,
    Kickback, Level, LifeState, Luck, Magazine, MagazineSize, MeleeQuery, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent, OccupancyGrid, OccupancyMaintenancePlugin, PieceQuery, Position,
    ReloadTu, SeverityRng, ShooterQuery, Shooting, ShotRng, SlabLedger, Stable, Stance, StanceKind,
    SurfaceGrid, TargetQuery, Toughness, Tu, TuMax, WeaponBundle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponQuery, WeaponShred, WearsQuery, WieldedBy, WieldsQuery, WornBy, Wounds,
    fire::FireOrder,
};

/// The faithful skirmish-style geometry the contract names: a shooter near (5,6).
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

/// The enemy's cell near (12,9).
fn enemy_cell() -> CellLevel {
    CellLevel::new(Cell::new(12, 9), Level::new(0))
}

/// One single-shot fire-mode spec (arbitrary, non-pinned per-mode numbers).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// Equip a ganger's six worn-armor-piece entities (one per [`BodyPart`]) at the thin
/// uniform stats, related via `WornBy` (GTW-323 / ADR-0004). `fire()` resolves the
/// struck location through `ganger → Wears → the BodyPart-tagged piece`, so a target
/// must carry its pieces; the relationship hook populates `Wears` synchronously in a
/// bare `World` spawn.
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
/// `sync_moved_gangers` is the production system that publishes the occupant band),
/// plus the sim resources `fire()` reads. NO `set_occupant_band` call anywhere.
fn landed_hit_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OccupancyMaintenancePlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app
}

/// Spawn an armed, alive, loaded, aiming PLAYER-faction shooter at the shooter cell
/// facing the enemy — the full `ShooterQuery` (`With<Weapon>` + weapon stats) AND
/// `TargetQuery` (its own liveness reads through the target query) set. A tight
/// `BaseSpread` + high `Accuracy` + aiming so the cone clusters on the enemy and
/// some seeds land.
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
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode()]),
            Stable::new(true),
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
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`), not the
    // ganger; the `WieldedBy` insert hook populates the ganger's `Wields` synchronously
    // in a bare `World` spawn so the very next `fire()` resolves it.
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// Spawn a STANDING enemy ganger at the enemy cell with a real `Position` + `Stance`
/// (so the production `sync_moved_gangers` publishes its silhouette band) carrying
/// the full `TargetQuery` battle-surface set.
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

/// Run ONE `fire()` volley at the enemy with a fresh seed `seed`, returning whether
/// the enemy took an effect (Hp dropped from its pre-fire value OR an
/// `InflictedWound` was recorded). Reads the maintained grids straight off the world.
fn one_volley_lands(app: &mut App, shooter: Entity, enemy: Entity, seed: u64) -> bool {
    let hp_before = app.world().get::<Hp>(enemy).map_or(0, |h| **h);
    let wounds_before = app
        .world()
        .get::<InflictedWounds>(enemy)
        .map_or(0, |w| w.len());

    let tuning = CombatTuning::default();
    let mut shot_rng = ShotRng::from_root(BattleSeed::new(seed));
    let mut sev_rng = SeverityRng::from_root(BattleSeed::new(seed));
    let mut injury_rng = InjuryRng::from_root(BattleSeed::new(seed));
    let mode = single_mode();

    // Snapshot the maintained grids (cloned read views) so the fire's two disjoint
    // queries can borrow the world mutably without aliasing the resources.
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
        MeleeQuery,
    )> = SystemState::new(app.world_mut());
    {
        let world = app.world_mut();
        // `get_mut` returns a `Result` (Bevy 0.19); the params always validate, so
        // `Err` is structurally impossible — returning `false` would fail the
        // calling assertion loudly rather than silently skip the fire.
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee)) =
            state.get_mut(world)
        else {
            return false;
        };
        let _volley = gdtf_battle_sim::fire(
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

/// GTW-304 — a real ganger shot LANDS: with the occupant band published by the
/// production `sync_moved_gangers` (NOT by hand), firing at the enemy across several
/// seeds drops its Hp / records a wound on at least one seed. On the pre-fix code
/// the band is never published, so NO seed lands and this test fails.
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
    // GTW-323: equip each ganger's worn-armor PIECE entities (the combat read+wear
    // path that `fire()` resolves through `Wears`).
    equip_thin_armor(&mut app, shooter);
    equip_thin_armor(&mut app, enemy);

    // ONE update: the production occupancy-maintenance chain runs. `sync_moved_gangers`
    // sees both freshly-spawned Positions as `Changed` (first-run semantics) and
    // publishes each occupant's stance-derived silhouette band off the grid — the
    // band is published by REAL code, never by `set_occupant_band` here.
    app.update();

    // Sanity: the enemy's band IS published (the fix) — a standing ganger presents
    // the HIGH band. (Pre-fix this reads `None`.)
    let enemy_band = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&enemy_cell()));
    assert!(
        enemy_band.is_some(),
        "the production sync must publish the enemy's occupant band (GTW-304); got {enemy_band:?}",
    );

    // Fire across several seeds; a LANDED hit on >= 1 seed proves rounds now register.
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
         through the ganger (GTW-304).",
        seeds.len(),
    );
}
