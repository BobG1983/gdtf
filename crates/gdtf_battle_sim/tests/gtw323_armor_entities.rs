//! GTW-323 slice 1 (ADR-0004) — armor as related entities, end-to-end on the REAL
//! `setup_battle` → `fire()` path.
//!
//! Proves the two clauses the relationship remodel owns:
//!
//! 1. **Struck-piece read + wear go through the piece ENTITY** — after a `fire()`
//!    volley lands on a target spawned by `setup_battle` (whose armor lives on related
//!    `Wears` piece entities, NOT a `WornArmor` array), the struck location's
//!    `ArmorIntegrity` **component** has dropped: the wear mutated the piece entity, the
//!    new combat surface.
//! 2. **Determinism holds** — two identical seeded `setup_battle` + volley replays
//!    produce a byte-equal `Volley` (the keyed `ganger → Wears → BodyPart-tagged piece`
//!    lookup is allocation-order-independent, so the value a struck part resolves to is
//!    identical regardless of entity storage; ADR-0004 §Determinism).
//!
//! The setup spawns gangers as `bsn!` scenes whose `Wears` pieces materialize on the
//! `SpawnScene` schedule, so each test `app.update()`s once after setup before firing.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::{RunSystemOnce, SystemState},
    prelude::{Commands, Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Aim, Aiming, ArmorIntegrity, BattleGrids, BattleRegistries, BattleSeed, BattleSetup,
    BraceStairCells, Cell, CombatTuning, CoverLedger, Direction, Facing, Faction, FireModeSpec,
    GangerStatTuning, InjuryRegistry, InjuryRng, InjuryTables, Level, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent, OccupancyGrid, PieceQuery, SeverityRng, ShooterQuery, ShotKind,
    ShotRng, SlabLedger, Stance, StanceKind, SurfaceGrid, TargetQuery, Volley, WeaponQuery, Wears,
    WearsQuery, WieldsQuery,
    fire::FireOrder,
    setup_battle,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, key, test_armor_registry, test_weapon_registry,
    },
};

/// One single-shot fire-mode spec (arbitrary, non-pinned per-mode numbers).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// The shooter's cell and the enemy's cell — a few cells apart, enemy due East.
const fn shooter_at() -> bevy::math::IVec2 {
    bevy::math::IVec2::new(5, 6)
}
const fn enemy_at() -> bevy::math::IVec2 {
    bevy::math::IVec2::new(9, 6)
}

/// Build the real battle via `setup_battle` against the central test registries — a
/// `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` app (the scene infrastructure the
/// `bsn!` ganger + `Wears` piece spawns need), driven once with `app.update()` so the
/// deferred scenes materialize. Returns `(app, setup)` or `None` on a setup failure.
///
/// The shooter is an East-facing, aiming, high-Shooting + tight-`BaseSpread` ganger so
/// the cone clusters on the enemy; the enemy is a standing ganger directly East. Both
/// resolve the central test weapon + armor keys (a paper-thin suit, so a landed round
/// reliably wears the struck piece).
fn battle_app() -> Option<(App, BattleSetup)> {
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([
            // Shooter: tight cone, aiming, high Aim → high DERIVED Shooting (GTW-384:
            // Shooting = aim·Aim + reflexes·Reflexes + cool·Cool) — lands reliably.
            GangerSpawnBuilder::new()
                .at(key(shooter_at().x, shooter_at().y, 0))
                .faction(Faction::new(0))
                .facing(Facing::new(Direction::East))
                .stance(Stance::new(StanceKind::Standing))
                .aiming(Aiming::new(true))
                .aim(Aim::new(10.0))
                .build(),
            // Enemy: standing, due East — the target the shooter aims at.
            GangerSpawnBuilder::new()
                .at(key(enemy_at().x, enemy_at().y, 0))
                .faction(Faction::new(1))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    let registry = test_weapon_registry();
    let armor = test_armor_registry();
    let terrain = gdtf_battle_sim::test_support::test_terrain_registry();
    // GTW-384: setup derives each ganger's computed stats from the default stat tuning.
    let stat_tuning = GangerStatTuning::default();
    // GTW-396: fallback floor cost (no default_floor authored in test fixtures).
    let fallback_floor_cost = gdtf_battle_sim::tuning::CombatTuning::default()
        .move_costs
        .open;
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, Some(&terrain)),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(setup.is_some(), "setup_battle must succeed");
    let setup = setup?;
    // Materialize the deferred `bsn!` ganger + `Wears` piece scenes.
    app.update();
    // A second update so the production occupancy-maintenance is settled and the
    // materialized Positions are seen (the grids were seeded by setup directly).
    app.update();
    Some((app, setup))
}

/// Drive ONE seeded `fire()` volley from `shooter` at the enemy cell, reading the
/// grids `setup_battle` inserted as resources. Returns the frozen `Volley`.
fn fire_once(app: &mut App, shooter: Entity, seed: u64) -> Volley {
    let tuning = CombatTuning::default();
    let mut rng = ShotRng::from_root(BattleSeed::new(seed));
    let mut sev_rng = SeverityRng::from_root(BattleSeed::new(seed));
    let mut injury_rng = InjuryRng::from_root(BattleSeed::new(seed));
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
    let mode = single_mode();

    let mut state: SystemState<(
        ShooterQuery,
        TargetQuery,
        WearsQuery,
        PieceQuery,
        WieldsQuery,
        WeaponQuery,
    )> = SystemState::new(app.world_mut());
    let volley = {
        let world = app.world_mut();
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons)) =
            state.get_mut(world)
        else {
            return Volley {
                reports: Vec::new(),
                shots:   Vec::new(),
            };
        };
        gdtf_battle_sim::fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(enemy_at().x, enemy_at().y),
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
            &InjuryTables::default(),
            &InjuryRegistry::default(),
            &mut injury_rng,
        )
    };
    state.apply(app.world_mut());
    volley
}

/// The minimum `ArmorIntegrity` across a ganger's worn pieces — read off the piece
/// ENTITY components (the combat surface), so a drop proves the wear hit the entity.
fn min_piece_integrity(app: &App, ganger: Entity) -> Option<i32> {
    let wears = app.world().get::<Wears>(ganger)?;
    bevy::ecs::relationship::RelationshipTarget::iter(wears)
        .filter_map(|piece| app.world().get::<ArmorIntegrity>(piece).map(|c| **c))
        .min()
}

/// GTW-323 (clause 3) — a landed `fire()` volley wears the struck piece ENTITY's
/// `ArmorIntegrity` component (not a `WornArmor` array slot). Fires repeated seeds
/// until a round lands on the enemy (a `Ganger` outcome), then asserts the enemy's
/// minimum worn-piece integrity dropped below its spawned value — proving the read +
/// wear flowed through the related piece entity.
#[test]
fn a_landed_shot_wears_the_struck_piece_entity_integrity() {
    let Some((mut app, setup)) = battle_app() else {
        return;
    };
    // setup.occupants is in authored order: [shooter, enemy].
    assert_eq!(
        setup.occupants.len(),
        2,
        "the setup must spawn both the shooter and the enemy",
    );
    let (Some(shooter_p), Some(enemy_p)) = (setup.occupants.first(), setup.occupants.get(1)) else {
        return;
    };
    let shooter = shooter_p.occupant;
    let enemy = enemy_p.occupant;

    let before = min_piece_integrity(&app, enemy);
    assert!(
        before.is_some(),
        "the enemy must carry worn-piece integrity components (the Wears entities)",
    );

    // Fire across many seeds until at least one volley LANDS on the enemy (a Ganger
    // outcome). The struck piece's integrity then must be below its spawned value.
    let mut landed = false;
    for seed in 0..256u64 {
        let volley = fire_once(&mut app, shooter, seed);
        let hit_ganger = volley
            .reports
            .iter()
            .any(|r| matches!(r.kind, ShotKind::Ganger(e) if e == enemy));
        if hit_ganger {
            landed = true;
            break;
        }
    }
    assert!(
        landed,
        "across the seed sweep at least one volley must land on the enemy (a Ganger hit)",
    );

    let after = min_piece_integrity(&app, enemy);
    assert!(
        matches!((before, after), (Some(b), Some(a)) if a < b),
        "a landed shot must wear the struck piece ENTITY's ArmorIntegrity below its \
         spawned value (before={before:?} after={after:?}) — the wear went through the \
         related piece entity, not a WornArmor array slot",
    );
}

/// GTW-323 (clause 5) — seeded determinism holds through the armor-relationship
/// remodel: two identical `setup_battle` + same-seed `fire()` runs produce a byte-equal
/// `Volley`. The keyed `ganger → Wears → BodyPart-tagged piece` lookup is
/// allocation-order-independent, so the struck-piece value (and therefore the whole
/// fire result) is reproduced exactly (ADR-0004 §Determinism — cheap insurance).
#[test]
fn same_seed_reproduces_a_byte_equal_volley_through_the_relationship() {
    const SEED: u64 = 0xA12_0323;

    let run = || {
        let (mut app, setup) = battle_app()?;
        let shooter = setup.occupants.first().map(|p| p.occupant)?;
        Some(fire_once(&mut app, shooter, SEED))
    };

    let (first, second) = (run(), run());
    assert!(
        first.is_some() && second.is_some(),
        "both seeded runs must build a battle + fire a volley",
    );
    assert_eq!(
        first, second,
        "the same battle seed must reproduce a byte-equal Volley through the \
         armor-relationship lookup (determinism held — ADR-0004)",
    );
}
