//! Integration tests for the E2.9 coarse-pipeline entry point (GTW-172).
//!
//! These exercise the **public** `resolve_coarse` surface as a downstream user
//! would: build a battle (via `setup_battle` for the on-the-real-path geometry, or
//! hand-built occupancy / surface / cover for the precise miss / cover / ground
//! geometry), call `resolve_coarse`, and assert on the returned `ShotOutcome`.
//!
//! Every position the resolver touches is in **sim units** (cubic-voxel `SimPos` /
//! a unit-`Vec3` direction) — zero pixels. The cone width and concentration are
//! fed in already composed (the ticket's composed inputs); the geometry tests use
//! a **zero cone** so the sample is dead-center on the aim axis (deterministic) and
//! a HIGH occupant band so any round impacts, keeping the hand-computed geometry
//! robust against the tunable aim/muzzle fractions.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins, World},
    scene::ScenePlugin,
};
// The canonical shared builders + specs + registries + the `(cell, level)` key
// helper (consolidated out of this file's former local copies — GTW-324 migration).
use gdtf_battle_sim::test_support::{
    GangerSpawnBuilder, SituationBuilder, ganger_at, key, shot_rng, test_armor_registry,
    test_weapon_registry,
};
use gdtf_battle_sim::{
    Accuracy, Aim, Aiming, ArmorHardness, ArmorProtection, BattleRegistries, BattleSetup, BodyPart,
    Cell, CellLevel, CombatTuning, ConcentrationP, ConeAngle, CoverEntry, CoverHp, CoverLedger,
    Direction, Facing, Faction, GangerStatTuning, HeightBand, Hp, Level, OccupancyGrid, Position,
    PriorShots, RecoilClimb, RecoilGrowth, Shooting, ShotInputs, ShotKind, ShotOutcome, Situation,
    Stance, StanceKind, SurfaceGrid, Tu, Wounds, concentration_p, resolve_coarse, setup_battle,
};

/// Run `setup_battle` on a fresh app, drive the `SpawnScene` schedule so the deferred
/// `bsn!` ganger components materialize (GTW-322), and return the app + `BattleSetup`
/// — or `None` on failure (keeping the tests free of `unwrap`/`expect`/`panic`, all
/// denied in tests too).
fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    // `AssetPlugin` + `ScenePlugin` are required: `setup_battle` now spawns each ganger
    // as a `bsn!` Scene (GTW-322), whose deferred materialization needs the scene/asset
    // infrastructure.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));

    let gangs = gdtf_battle_sim::test_support::test_gang_registry();
    let registry = test_weapon_registry();
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup.
    let melee = gdtf_battle_sim::test_support::test_melee_weapon_registry();
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
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(setup.is_some(), "setup_battle must succeed on the fixture");
    let setup = setup?;
    // Only `app.update()` runs the `SpawnScene` schedule that turns the queued ganger
    // scenes into live components the per-entity `combat_snapshot` reads (GTW-322).
    app.update();
    Some((app, setup))
}

/// A zero cone angle — the sample is the aim axis EXACTLY (dead-center), so the
/// geometry is deterministic regardless of the RNG stream.
const fn zero_cone() -> ConeAngle {
    ConeAngle::new(0.0)
}

/// An arbitrary concentration exponent (NOT a shipped magnitude); irrelevant under
/// a zero cone but a real value for the seeded-replay test.
const fn some_p() -> ConcentrationP {
    ConcentrationP::new(2.0)
}

/// The first shot of an action with zero recoil climb / growth — the central axis
/// is the untilted muzzle→aim axis exactly.
const fn no_recoil() -> (PriorShots, RecoilClimb, RecoilGrowth) {
    (
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
}

/// The no-op corpse predicate (GTW-317) — marks no occupant dead, so every occupant
/// still stops the round. These `resolve_coarse` tests exercise the coarse pipeline
/// over live occupants only, so the dead-skip is never engaged here.
fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}

/// Bundle the per-shot description into a [`ShotInputs`] (GTW-179): a standing East
/// shooter at `shooter_at` firing at a standing target at `target_at`, with the
/// given `cover_band` / cone width and zero recoil (the first shot). This is the
/// GTW-172 call surface, rewrapped — the geometry is identical.
const fn standing_shot(
    shooter_at: CellLevel,
    target_at: CellLevel,
    cover_band: Option<HeightBand>,
    cone: ConeAngle,
) -> ShotInputs {
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();
    ShotInputs {
        shooter_position: Position::new(shooter_at),
        shooter_facing: Facing::new(Direction::East),
        shooter_stance: Stance::new(StanceKind::Standing),
        target_position: Position::new(target_at),
        target_stance: Stance::new(StanceKind::Standing),
        cover_band,
        cone,
        p: some_p(),
        prior_shots,
        recoil_climb,
        recoil_growth,
    }
}

// --- AC #2 — the real path yields a Ganger outcome whose Entity is the target. ---

/// `resolve_coarse` over a `setup_battle` situation (the real path) yields a
/// `Ganger` outcome whose struck `Entity` is the target and whose `BodyPart` is
/// set: a standing shooter at (2,2,0) facing East, a standing target at (5,2,0)
/// banded HIGH so any round impacts.
#[test]
fn resolve_coarse_yields_ganger_outcome_on_real_path() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(shooter_at, 0), ganger_at(target_at, 1)])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let target_entity: Entity = setup.occupants[1].occupant;

    // The change-driven occupancy sync (occupancy_sync::sync_moved_gangers, GTW-304)
    // publishes the occupant band on the real path; this isolated geometry test sets
    // the same maintained state directly via set_occupant_band (HIGH so any round
    // impacts), keeping the hand-computed geometry independent of the sync wiring.
    let world: &mut World = app.world_mut();
    let Some(mut grid) = world.get_resource_mut::<OccupancyGrid>() else {
        return;
    };
    grid.set_occupant_band(target_at, Some(HeightBand::High));

    let tuning = CombatTuning::default();
    let mut rng = shot_rng(0xA_11CE);

    let Some(occupancy) = world.get_resource::<OccupancyGrid>() else {
        return;
    };
    let Some(surface) = world.get_resource::<SurfaceGrid>() else {
        return;
    };
    let Some(cover) = world.get_resource::<CoverLedger>() else {
        return;
    };

    let shot = standing_shot(shooter_at, target_at, None, zero_cone());
    let outcome = resolve_coarse(
        &shot,
        occupancy,
        surface,
        cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert_eq!(
        outcome.kind,
        ShotKind::Ganger(target_entity),
        "the shot must strike the target ganger entity",
    );
    assert_eq!(
        outcome.cell,
        Cell::new(5, 2),
        "the outcome cell is the target"
    );
    assert_eq!(
        outcome.level,
        Level::new(0),
        "the outcome storey is the target's"
    );
    assert!(
        outcome.body_part.is_some(),
        "a ganger outcome carries a struck body part",
    );
}

/// GTW-384 C8(d) — the LIVE-COMBAT smoke: a ganger's DERIVED `Shooting` (computed at
/// `setup_battle` from its eight authored attributes × `GangerStatTuning`) drives shot
/// resolution AS BEFORE, and its DERIVED `Hp` / `Wounds` are the live combat pools. Spawn
/// via the REAL `setup_battle` derivation path, read the shooter's derived `Shooting` off
/// the entity, feed it through the SAME §1b `concentration_p` the fire pipeline uses, and
/// run `resolve_coarse` with the resulting `ConcentrationP` — it must land on the target
/// ganger (the derived skill drives the shot). The target's derived `Hp` / `Wounds` are
/// the pools `apply_hit` drains. RELATIONS-ONLY: asserts the derived stats are usable
/// (`> 0`) and drive a landed outcome — never a pinned shipped magnitude.
#[test]
fn derived_shooting_and_pools_drive_combat_resolution() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    // High-Aim shooter (→ high derived Shooting) vs a default-attribute target.
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(shooter_at)
                .faction(Faction::new(0))
                .facing(Facing::new(Direction::East))
                .stance(Stance::new(StanceKind::Standing))
                .aiming(Aiming::new(true))
                .aim(Aim::new(12.0))
                .build(),
            ganger_at(target_at, 1),
        ])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let shooter_entity: Entity = setup.occupants[0].occupant;
    let target_entity: Entity = setup.occupants[1].occupant;

    // The shooter's DERIVED Shooting — read off the spawned entity (the value setup
    // derived from the authored attributes). It must be a usable, positive skill.
    let shooting = app.world().get::<Shooting>(shooter_entity).copied();
    assert!(
        shooting.is_some_and(|s| *s > 0.0),
        "the shooter's DERIVED Shooting must be a usable positive skill: {shooting:?}",
    );
    let Some(shooting) = shooting else { return };

    // The target's DERIVED life pools — the combat drain pools. Both positive (a fresh
    // ganger has HP + Wounds to lose).
    let target_hp = app.world().get::<Hp>(target_entity).copied();
    let target_wounds = app.world().get::<Wounds>(target_entity).copied();
    assert!(
        target_hp.is_some_and(|h| *h > 0),
        "the target's DERIVED Hp must be a positive knock-down pool: {target_hp:?}",
    );
    assert!(
        target_wounds.is_some_and(|w| *w > 0),
        "the target's DERIVED Wounds must be a positive life pool: {target_wounds:?}",
    );

    let world: &mut World = app.world_mut();
    let Some(mut grid) = world.get_resource_mut::<OccupancyGrid>() else {
        return;
    };
    grid.set_occupant_band(target_at, Some(HeightBand::High));

    let tuning = CombatTuning::default();
    // Feed the DERIVED Shooting through the SAME §1b concentration the fire pipeline uses.
    let p = concentration_p(
        shooting,
        Accuracy::new(1.0),
        tuning.cone_stability.concentration,
    );
    assert!(
        *p > 0.0,
        "the derived Shooting yields a positive concentration exponent"
    );

    let mut rng = shot_rng(0xA_11CE);
    let Some(occupancy) = world.get_resource::<OccupancyGrid>() else {
        return;
    };
    let Some(surface) = world.get_resource::<SurfaceGrid>() else {
        return;
    };
    let Some(cover) = world.get_resource::<CoverLedger>() else {
        return;
    };

    let shot = standing_shot(shooter_at, target_at, None, zero_cone());
    let outcome = resolve_coarse(
        &shot,
        occupancy,
        surface,
        cover,
        &tuning,
        &mut rng,
        no_dead(),
    );
    assert_eq!(
        outcome.kind,
        ShotKind::Ganger(target_entity),
        "the DERIVED-Shooting shot lands on the target ganger (the derived skill drives the shot)",
    );
}

// --- AC #4 — Miss / Cover / Ground over composed geometry. ---

/// A shot that clears everything returns a `Miss` (no struck object), and the
/// trajectory is still carried for presenter FX. Empty grids, flat East: nothing in
/// the path, the round leaves laterally.
#[test]
fn clearing_shot_returns_miss_carrying_trajectory() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut rng = shot_rng(7);

    let shot = standing_shot(key(2, 2, 0), key(5, 2, 0), None, zero_cone());
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert_eq!(
        outcome.kind,
        ShotKind::Miss,
        "an empty path clears to a Miss"
    );
    assert!(
        outcome.body_part.is_none(),
        "a non-ganger outcome carries no body part",
    );
    // The trajectory is a real unit vector (carried for FX even on a miss).
    let traj = outcome.trajectory.vec();
    assert!(
        (traj.length() - 1.0).abs() < 1.0e-4,
        "the trajectory is a unit direction, carried even on a miss",
    );
}

/// A shot into cover returns a `Cover` outcome carrying the struck `CoverEntry`. A
/// MID cover stands in the path; the round (dead-center on the aim axis) impacts it.
#[test]
fn shot_into_cover_returns_cover_entry() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();

    let cover_cell = key(5, 2, 0);
    let entry = CoverEntry::seeded(
        CoverHp::new(80),
        HeightBand::High,
        ArmorProtection::new(6),
        ArmorHardness::new(3),
    );
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell, entry);

    let mut rng = shot_rng(99);

    // Aim at the cover cell's own band midpoint (a deliberately-shot crate).
    let shot = standing_shot(
        key(2, 2, 0),
        cover_cell,
        Some(HeightBand::High),
        zero_cone(),
    );
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert_eq!(
        outcome.kind,
        ShotKind::Cover(entry),
        "the shot must strike the cover, carrying its CoverEntry",
    );
    assert!(
        outcome.body_part.is_none(),
        "a cover outcome carries no body part",
    );
}

/// A downward shot off the grid bottom returns a `Ground` outcome carrying the
/// surface cell it exited through. Straight down (-z) from (4,4,0).
#[test]
fn downward_shot_off_bottom_returns_ground() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut rng = shot_rng(13);
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();

    // Target directly below the shooter so the muzzle→aim axis points down (-z).
    // North-facing (not the East helper) to keep this geometry identical to GTW-172.
    let shot = ShotInputs {
        shooter_position: Position::new(key(4, 4, 1)),
        shooter_facing: Facing::new(Direction::North),
        shooter_stance: Stance::new(StanceKind::Standing),
        target_position: Position::new(key(4, 4, 0)),
        target_stance: Stance::new(StanceKind::Standing),
        cover_band: None,
        cone: zero_cone(),
        p: some_p(),
        prior_shots,
        recoil_climb,
        recoil_growth,
    };
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert!(
        matches!(outcome.kind, ShotKind::Ground(_)),
        "a shot diving off the grid bottom strikes the Ground, got {:?}",
        outcome.kind,
    );
    assert!(
        outcome.body_part.is_none(),
        "a ground outcome carries no body part",
    );
}

// --- AC #5 — determinism (same seed → same outcome). ---

/// The SAME `BattleSeed` yields the SAME `ShotOutcome` for identical inputs — a
/// seeded-replay property. Uses a NON-zero cone so the cone sample (and the ganger
/// part roll) genuinely draw from the RNG, then asserts two fresh seeds reproduce.
#[test]
fn same_seed_yields_same_outcome() {
    let tuning = CombatTuning::default();
    let target = key(5, 2, 0);
    let mut occupancy = OccupancyGrid::new();
    // Hand-build the maintained occupant: a HIGH-banded ganger so any round impacts.
    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();
    occupancy.set_occupant(target, Some(entity));
    occupancy.set_occupant_band(target, Some(HeightBand::High));
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    // A non-zero cone so the sample genuinely draws (radius + azimuth) and the part
    // roll draws too — both must reproduce under the same seed.
    let shot = standing_shot(key(2, 2, 0), target, None, ConeAngle::new(0.05));
    let run = |seed: u64| -> ShotOutcome {
        let mut rng = shot_rng(seed);
        resolve_coarse(
            &shot,
            &occupancy,
            &surface,
            &cover,
            &tuning,
            &mut rng,
            no_dead(),
        )
    };

    let a = run(0xC0_FFEE);
    let b = run(0xC0_FFEE);
    assert_eq!(a, b, "the same seed reproduces the same ShotOutcome");
    // The replay struck the ganger and rolled a part (the RNG path actually ran).
    assert_eq!(a.kind, ShotKind::Ganger(entity));
    assert!(
        a.body_part.is_some(),
        "the ganger replay carries a body part"
    );
}

// --- AC #6 — no mutation; sim-unit positions; no damage / TU bookkeeping. ---

/// A snapshot of the combat state `resolve_coarse` must NOT touch — the target's
/// `Hp` / `Wounds` / worn-piece `ArmorIntegrity` (E3, on the related `Wears` piece
/// entities since GTW-323) and the shooter's `Tu` (E4).
struct CombatSnapshot {
    hp:     Hp,
    wounds: Wounds,
    /// The target's worn-piece integrities (sorted) — read off the `Wears` piece
    /// entities (GTW-323 / ADR-0004), not a removed `WornArmor` component.
    armor:  Vec<i32>,
    tu:     Tu,
}

/// Read the [`CombatSnapshot`] for `(target, shooter)` off the world, or `None` if
/// any component is missing (keeping the test panic-free).
fn combat_snapshot(world: &World, target: Entity, shooter: Entity) -> Option<CombatSnapshot> {
    use bevy::ecs::relationship::RelationshipTarget;

    let wears = world.get::<gdtf_battle_sim::Wears>(target)?;
    let mut armor: Vec<i32> = wears
        .iter()
        .filter_map(|piece| {
            world
                .get::<gdtf_battle_sim::ArmorIntegrity>(piece)
                .map(|c| **c)
        })
        .collect();
    armor.sort_unstable();
    Some(CombatSnapshot {
        hp: *world.get::<Hp>(target)?,
        wounds: *world.get::<Wounds>(target)?,
        armor,
        tu: *world.get::<Tu>(shooter)?,
    })
}

/// `resolve_coarse` mutates NOTHING: after the call the struck ganger's `Hp` /
/// `Wounds` / worn-piece integrities and the shooter's `Tu` are unchanged (E3 / E4
/// boundary).
#[test]
fn resolve_coarse_mutates_no_combat_state() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(shooter_at, 0), ganger_at(target_at, 1)])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let shooter_entity: Entity = setup.occupants[0].occupant;
    let target_entity: Entity = setup.occupants[1].occupant;

    let Some(before) = combat_snapshot(app.world(), target_entity, shooter_entity) else {
        return;
    };

    // Publish the maintained occupant band (HIGH so any round impacts).
    {
        let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
            return;
        };
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    // Resolve a shot at the target; assert it struck (so the no-mutation claim is
    // about a real hit) and the trajectory is a sim-space unit vector (zero px).
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(0xBEEF);
    {
        let world: &World = app.world();
        let (Some(occupancy), Some(surface), Some(cover)) = (
            world.get_resource::<OccupancyGrid>(),
            world.get_resource::<SurfaceGrid>(),
            world.get_resource::<CoverLedger>(),
        ) else {
            return;
        };
        let shot = standing_shot(shooter_at, target_at, None, zero_cone());
        let outcome = resolve_coarse(
            &shot,
            occupancy,
            surface,
            cover,
            &tuning,
            &mut rng,
            no_dead(),
        );
        assert_eq!(
            outcome.kind,
            ShotKind::Ganger(target_entity),
            "the shot struck the target (so the no-mutation claim is about a real hit)",
        );
        let traj = outcome.trajectory.vec();
        assert!(
            (traj.length() - 1.0).abs() < 1.0e-4,
            "the trajectory is a sim-space unit direction",
        );
    }

    // The combat state is unchanged — no damage (E3), no TU spend (E4).
    let Some(after) = combat_snapshot(app.world(), target_entity, shooter_entity) else {
        return;
    };
    assert_eq!(
        after.hp, before.hp,
        "the target's Hp is unchanged (no damage)"
    );
    assert_eq!(
        after.wounds, before.wounds,
        "the target's Wounds are unchanged (no severity)",
    );
    assert_eq!(
        after.armor, before.armor,
        "the target's worn-piece integrities are unchanged (no degradation)",
    );
    assert_eq!(
        after.tu, before.tu,
        "the shooter's Tu is unchanged (no fire economy)"
    );
}

// --- AC #3 — the passed grid is READ, not rebuilt. ---

/// `resolve_coarse` reads the passed grids — the SAME hand-built grid instance
/// drives the outcome (no per-shot rebuild path exists in the signature). Marking a
/// HIGH occupant in the grid we pass produces a Ganger outcome carrying THAT
/// entity; an empty grid over the same geometry produces a Miss — so the result
/// provably came from the grid we handed in, not a fresh internal one.
#[test]
fn resolve_coarse_reads_the_passed_grid() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let target = key(5, 2, 0);

    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();

    // Grid A: a HIGH occupant marked at the target (the maintained state).
    let mut grid_with_occupant = OccupancyGrid::new();
    grid_with_occupant.set_occupant(target, Some(entity));
    grid_with_occupant.set_occupant_band(target, Some(HeightBand::High));
    // Grid B: empty (no occupant).
    let grid_empty = OccupancyGrid::new();

    let shot = standing_shot(key(2, 2, 0), target, None, zero_cone());
    let run = |grid: &OccupancyGrid| -> ShotOutcome {
        let mut rng = shot_rng(1);
        resolve_coarse(&shot, grid, &surface, &cover, &tuning, &mut rng, no_dead())
    };

    let with_occupant = run(&grid_with_occupant);
    let empty = run(&grid_empty);

    assert_eq!(
        with_occupant.kind,
        ShotKind::Ganger(entity),
        "the passed grid's occupant drives the outcome — the grid was READ",
    );
    assert_eq!(
        empty.kind,
        ShotKind::Miss,
        "an empty passed grid yields a Miss — no internal rebuild conjured an occupant",
    );
}

// --- AC #1 — each ShotOutcome variant constructs and inspects. ---

/// Each `ShotKind` variant constructs and its struck payload reads back, and a
/// `ShotOutcome` carries every named field — the type-surface check (AC #1).
#[test]
fn shot_outcome_variants_construct_and_inspect() {
    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();
    let entry = CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    );
    let surface_cell = key(3, 4, 2);

    // Ganger variant — carries the Entity; the body part is Some only here.
    let ganger = ShotOutcome {
        kind:       ShotKind::Ganger(entity),
        cell:       Cell::new(3, 4),
        level:      Level::new(2),
        body_part:  Some(BodyPart::Torso),
        band:       HeightBand::Mid,
        muzzle:     gdtf_battle_sim::SimPos::new(2.5, 2.5, 0.5),
        trajectory: trajectory_unit_x(),
    };
    assert_eq!(ganger.kind, ShotKind::Ganger(entity));
    assert_eq!(ganger.body_part, Some(BodyPart::Torso));
    assert_eq!(ganger.cell, Cell::new(3, 4));
    assert_eq!(ganger.level, Level::new(2));

    // Cover / Slab / Ground / Miss variants carry their struck object (or none).
    assert!(matches!(ShotKind::Cover(entry), ShotKind::Cover(_)));
    assert_eq!(ShotKind::Slab(surface_cell), ShotKind::Slab(surface_cell));
    assert_eq!(
        ShotKind::Ground(surface_cell),
        ShotKind::Ground(surface_cell)
    );
    assert_eq!(ShotKind::Miss, ShotKind::Miss);
}

/// A unit-X `ShotDir` for the type-surface test — built through the public cone
/// sampler under a zero cone (dead-center on a +X aim axis), so it exercises the
/// real `ShotDir` constructor rather than a hand-built private value.
fn trajectory_unit_x() -> gdtf_battle_sim::ShotDir {
    use gdtf_battle_sim::{SimPos, climb_aim_dir, sample_cone_vector};
    let mut rng = shot_rng(0);
    let axis = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.0),
        SimPos::new(1.0, 0.0, 0.0),
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    sample_cone_vector(axis, ConeAngle::new(0.0), some_p(), rng.rng())
}
