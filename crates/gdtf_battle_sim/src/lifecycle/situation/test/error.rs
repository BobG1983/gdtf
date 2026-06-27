//! Abort-first error tests — a bad vertical link and a missing weapon key each
//! return the typed error and spawn nothing (the no-panic contract).

use super::support::*;

/// A bad authored vertical link (dangling endpoint) makes `setup_battle` return
/// `Err(DanglingCell)` and spawn NOTHING — validation runs first, aborting the
/// setup before any entity or resource is created (the no-panic contract).
#[test]
fn setup_aborts_on_invalid_vertical_link() {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored — `missing` dangles
        .vertical_link(link)
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — these
    // tests abort BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    // The one-shot system ran; the inner setup returned the typed error (now wrapped
    // in BattleSetupError::InvalidLink — GTW-257).
    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::InvalidLink(
            InvalidVerticalLink::DanglingCell { link },
        )),
        "an invalid vertical link must abort setup with the typed error",
    );

    // Nothing was spawned (validation aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a validation abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a validation abort must insert no resources",
    );
}

/// GTW-257 AC3 — a `weapon` key ABSENT from the registry makes `setup_battle` return
/// `Err(BattleSetupError::WeaponNotFound)` and spawn NOTHING (resolution runs before
/// the spawn loop) — the no-panic handled-error contract. The dangling-link abort
/// precedent, for the weapon resolution.
#[test]
fn setup_errors_on_a_missing_weapon_key() {
    // A ganger whose weapon key is not the one the registry holds — overridden
    // off the central default via the builder's `.weapon()`.
    let ganger = GangerSpawnBuilder::new()
        .at(key(0, 0, 0))
        .faction(Faction::new(0))
        .weapon(WeaponName::new("no-such-weapon".to_owned()))
        .build();
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger)
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — these
    // tests abort BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::WeaponNotFound {
            weapon: WeaponName::new("no-such-weapon".to_owned()),
        }),
        "a missing weapon key must abort setup with WeaponNotFound (no panic)",
    );

    // Nothing was spawned (resolution aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a weapon-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a weapon-resolution abort must insert no resources",
    );
}

/// GTW-269 — an `armor` key ABSENT from the armor registry makes `setup_battle` return
/// `Err(BattleSetupError::ArmorNotFound)` and spawn NOTHING (armor resolution runs
/// before the spawn loop) — the no-panic handled-error contract, the armor mirror of
/// `setup_errors_on_a_missing_weapon_key`.
#[test]
fn setup_errors_on_a_missing_armor_key() {
    // A ganger whose armor key is not the one the armor registry holds (its weapon key
    // IS present — the builder default `TEST_WEAPON_KEY` — so the weapon resolution
    // passes and the armor resolution is reached). The bad armor key is overridden off
    // the central default via the builder's `.armor()`.
    let ganger = GangerSpawnBuilder::new()
        .at(key(0, 0, 0))
        .faction(Faction::new(0))
        .armor(ArmorName::new("no-such-armor".to_owned()))
        .build();
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger)
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — these
    // tests abort BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::ArmorNotFound {
            armor: ArmorName::new("no-such-armor".to_owned()),
        }),
        "a missing armor key must abort setup with ArmorNotFound (no panic)",
    );

    // Nothing was spawned (armor resolution aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "an armor-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "an armor-resolution abort must insert no resources",
    );
}

/// GTW-396 (gate clause A1) — a `walls` entry referencing a terrain piece KEY that is
/// ABSENT from the registry makes `setup_battle` return
/// `Err(BattleSetupError::TerrainNotFound)` and spawn NOTHING (terrain resolution runs
/// before the spawn loop) — the abort-first contract, the terrain mirror of
/// `setup_errors_on_a_missing_weapon_key`.
#[test]
fn setup_errors_on_a_missing_terrain_key() {
    // A ganger with the default (valid) weapon + armor keys, so the weapon + armor
    // resolution passes and the GTW-396 terrain resolution is reached. The wall names a
    // piece key that the test terrain registry does NOT hold.
    let missing = TerrainName::new("no-such-piece".to_owned());
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .with_scatter(CoverSpawn::new(key(2, 2, 0), missing.clone()))
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    // The registry DOES exist (so we reach key resolution) but lacks the authored key.
    let terrain = test_terrain_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, Some(&terrain)),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::TerrainNotFound { piece: missing }),
        "a missing terrain key must abort setup with TerrainNotFound (no panic)",
    );

    // Nothing was spawned, and no terrain ledger was inserted (terrain resolution
    // aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a terrain-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a terrain-resolution abort must insert no resources",
    );
    assert!(
        world.get_resource::<FloorCostGrid>().is_none(),
        "a terrain-resolution abort must insert no FloorCostGrid",
    );
}

/// GTW-414/415 — a [`PlacedGanger`]'s `gang` ref ABSENT from the
/// [`GangRegistry`](crate::ganger::GangRegistry) passed to `setup_battle` makes it return
/// `Err(BattleSetupError::GangNotFound)` and spawn NOTHING (the `(gang, member)` resolution
/// runs BEFORE the spawn loop — the abort-first invariant). The gang mirror of
/// `setup_errors_on_a_missing_weapon_key`. A regression replacing the `Err` with a
/// panic/unwrap, OR dropping abort-first (spawning a partial world before the error),
/// reddens this test.
#[test]
fn setup_errors_on_a_missing_gang_key() {
    // Build a valid one-ganger situation + its synthesized registry, then RE-POINT the
    // placement's `gang` ref at a name the synthesized registry does NOT hold — so the
    // registry passed to setup lacks the referenced gang (the post-build mutation idiom
    // the floor tests use to author a bad ref off a valid fixture).
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    let missing_gang = GangName::new("no-such-gang".to_owned());
    // Sanity: the situation HAS a placement to corrupt (else the resolution loop is empty
    // and the test would vacuously pass).
    assert!(
        !situation.gangers.is_empty(),
        "the fixture must field a ganger to re-point",
    );
    for placed in &mut situation.gangers {
        placed.gang = missing_gang.clone();
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — this
    // test aborts BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // No terrain authored in this fixture (the gang resolution fires first),
                // so terrain: None + the fallback floor cost suffices.
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::GangNotFound { gang: missing_gang }),
        "a missing gang ref must abort setup with GangNotFound (no panic), naming the gang",
    );

    // Nothing was spawned (the gang resolution aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a gang-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a gang-resolution abort must insert no resources",
    );
}

/// GTW-414/415 — a [`PlacedGanger`]'s `member` ref ABSENT from its (resolved) gang's
/// roster makes `setup_battle` return `Err(BattleSetupError::GangMemberNotFound)` and
/// spawn NOTHING. The gang DOES resolve (so the member-resolution branch is reached) —
/// only the member is missing — the member mirror of `setup_errors_on_a_missing_gang_key`.
/// A regression replacing the `Err` with a panic/unwrap, OR dropping abort-first
/// (spawning a partial world before the error), reddens this test.
#[test]
fn setup_errors_on_a_missing_member_key() {
    // Build a valid one-ganger situation + its synthesized registry, then RE-POINT the
    // placement's `member` ref at a name absent from its gang's roster — the gang itself
    // (untouched) still resolves, so setup reaches `roster.member(..)` and fails there.
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    let present_gang = GangName::new("gang_0".to_owned());
    let missing_member = GangerName::new("no-such-member".to_owned());
    // Sanity: the gang we leave the placement pointing at DOES resolve, so the failure is
    // provably the MEMBER lookup, not the gang lookup.
    assert!(
        gangs.roster(&present_gang).is_some(),
        "the synthesized registry must hold gang_0 (so the gang resolves, isolating the member)",
    );
    assert!(
        !situation.gangers.is_empty(),
        "the fixture must field a ganger to re-point",
    );
    for placed in &mut situation.gangers {
        placed.gang = present_gang.clone();
        placed.member = missing_member.clone();
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::GangMemberNotFound {
            gang:   present_gang,
            member: missing_member,
        }),
        "a missing member ref must abort setup with GangMemberNotFound (no panic), naming both",
    );

    // Nothing was spawned (the member resolution aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a member-resolution abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a member-resolution abort must insert no resources",
    );
}

/// GTW-396 (gate clause A2) — a `default_floor` piece whose `move_cost` is BELOW
/// [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST) (the A\* admissibility floor)
/// makes `setup_battle` return `Err(BattleSetupError::FloorCostBelowMinimum)` and spawn
/// NOTHING. A reverted / wrong-direction guard (`> minimum` instead of `< minimum`)
/// would let this through and fail the test.
#[test]
fn setup_errors_on_a_floor_cost_below_minimum() {
    // A registry whose ONE floor piece is authored below the admissibility floor. The
    // chosen cost (1) is an arbitrary discriminator strictly under MIN_MOVE_COST (4),
    // never a shipped magnitude.
    let too_cheap = TerrainName::new("too-cheap-floor".to_owned());
    let terrain = TerrainRegistry::new([(too_cheap.clone(), floor_spec(1))]);

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation.default_floor = too_cheap;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, Some(&terrain)),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    // Assert ONLY the variant (the magnitudes are arbitrary test data; the brittle-test
    // rule forbids pinning them): a too-cheap floor must abort with FloorCostBelowMinimum.
    assert!(
        matches!(
            setup_result.err(),
            Some(BattleSetupError::FloorCostBelowMinimum { .. })
        ),
        "a floor cost below MIN_MOVE_COST must abort with FloorCostBelowMinimum (no panic)",
    );

    // Nothing was spawned (the validation aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a below-minimum floor abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<FloorCostGrid>().is_none(),
        "a below-minimum floor abort must insert no FloorCostGrid",
    );
}

/// GTW-396 (gate clause A3) — a `default_floor` KEY that resolves to a NON-`Floor`
/// terrain spec (a Cover/Wall/Slab piece) makes `setup_battle` return
/// `Err(BattleSetupError::TerrainNotFound)` — the `resolve_floor_piece_cost` rejection
/// branch (the spec is present but the wrong KIND).
#[test]
fn setup_errors_when_floor_key_is_not_a_floor_spec() {
    // A registry whose key resolves to a COVER spec, not a Floor. Pointing `default_floor`
    // at it exercises the wrong-kind rejection branch.
    let not_a_floor = TerrainName::new("actually-cover".to_owned());
    let terrain = TerrainRegistry::new([(not_a_floor.clone(), cover_spec())]);

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation.default_floor = not_a_floor.clone();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, Some(&terrain)),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::TerrainNotFound { piece: not_a_floor }),
        "a floor key pointing at a non-Floor spec must abort with TerrainNotFound",
    );

    // Nothing was spawned (the validation aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a wrong-kind floor abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<FloorCostGrid>().is_none(),
        "a wrong-kind floor abort must insert no FloorCostGrid",
    );
}

/// GTW-396 (gate clause A4) — a non-empty `floors` OVERRIDE list makes the resulting
/// [`FloorCostGrid`] return the OVERRIDE cost for the overridden cell (not the default),
/// exercising the `overrides` map + `resolve_floor_costs` building `floor_overrides`.
/// The default and override costs are distinct arbitrary discriminators (each ≥
/// `MIN_MOVE_COST`), never shipped magnitudes.
#[test]
fn floor_override_cost_wins_over_default() {
    // Two distinct floor pieces, both ≥ MIN_MOVE_COST (4): a cheaper default (5) and a
    // dearer override (9). Arbitrary discriminators — the test asserts the OVERRIDE wins,
    // never a balance magnitude.
    let default_key = TerrainName::new("default-floor".to_owned());
    let override_key = TerrainName::new("override-floor".to_owned());
    let terrain = TerrainRegistry::new([
        (default_key.clone(), floor_spec(5)),
        (override_key.clone(), floor_spec(9)),
    ]);

    let overridden_cell = key(3, 3, 0);
    let plain_cell = key(8, 8, 0);
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation.default_floor = default_key;
    situation.floors = vec![FloorSpawn::new(overridden_cell, override_key)];

    let Some((mut app, _setup)) = run_setup_with(
        situation,
        gangs,
        test_registry(),
        test_armor_registry(),
        Some(&terrain),
    ) else {
        return;
    };

    let world = app.world_mut();
    let grid_present = world.get_resource::<FloorCostGrid>().is_some();
    assert!(
        grid_present,
        "a successful setup must insert a FloorCostGrid"
    );
    let Some(grid) = world.get_resource::<FloorCostGrid>() else {
        return;
    };
    // The overridden cell reads the OVERRIDE cost (9), NOT the default (5); a cell with
    // no override falls back to the default. The two differ, so a build that dropped the
    // override map (used the default everywhere) would fail this assertion.
    assert_eq!(
        grid.cost(&overridden_cell),
        MoveCost::new(9),
        "the overridden cell must read the override cost, not the default",
    );
    assert_eq!(
        grid.cost(&plain_cell),
        MoveCost::new(5),
        "a non-overridden cell must read the default cost",
    );
    assert_ne!(
        grid.cost(&overridden_cell),
        grid.cost(&plain_cell),
        "the override and default costs must differ (the override is observable)",
    );
}

/// GTW-457 — TWO authored gangers on the SAME `(cell, level)` make `setup_battle`
/// return `Err(BattleSetupError::StackedGangers { at })` naming the shared cell, and
/// spawn NOTHING (the pre-spawn dedup gate runs BEFORE the spawn loop — the abort-first
/// invariant). Both gangers carry VALID, resolvable `(gang, member)` refs (distinct
/// factions → distinct synthesized gangs/members), so the failure is provably the
/// STACK, not a missing ref. Pin-discriminating: with the gate removed, the GTW-156
/// last-write-wins occupancy pour would let both gangers spawn STACKED on one cell —
/// so this returns `Ok` (no error) and spawns two ganger entities, reddening every
/// assertion below.
#[test]
fn setup_errors_on_stacked_gangers() {
    let shared = key(5, 5, 0);
    // Two gangers on the SAME cell, on DISTINCT factions — `build_with_gangs` synthesizes
    // a registry where BOTH resolve (distinct gang per faction, distinct member name), so
    // the gang/member resolution passes and the StackedGangers gate is the sole cause.
    // The builder does not dedupe placements, preserving the stack the gate must reject.
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([ganger_at(shared, 0), ganger_at(shared, 1)])
        .build_with_gangs();
    // Sanity: the fixture really authors two gangers on the one cell (else the gate is
    // vacuously satisfied and the test proves nothing).
    assert_eq!(situation.gangers.len(), 2, "the fixture fields two gangers");
    assert!(
        situation.gangers.iter().all(|g| g.at == shared),
        "both gangers are authored on the one shared cell",
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — this
    // test aborts BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // No terrain authored (the dedup gate fires before terrain resolution),
                // so terrain: None + the fallback floor cost suffices.
                BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::StackedGangers { at: shared }),
        "two gangers on one cell must abort setup with StackedGangers, naming the cell",
    );

    // Nothing was spawned (the dedup gate aborted before the spawn loop) — neither ganger
    // entities (no Position) nor any resource. If the gate were removed, BOTH gangers
    // would spawn here (two Positions), so this count discriminates the gate.
    app.world_mut().flush();
    let world = app.world_mut();
    let mut positions = world.query::<&Position>();
    assert_eq!(
        positions.iter(world).count(),
        0,
        "a stacked-ganger abort must spawn no ganger entities",
    );
    let mut wears = world.query::<&Wears>();
    assert_eq!(
        wears.iter(world).count(),
        0,
        "a stacked-ganger abort must spawn no worn-armor entities",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a stacked-ganger abort must insert no resources",
    );
    assert!(
        world.get_resource::<OccupancyGrid>().is_none(),
        "a stacked-ganger abort must insert no OccupancyGrid",
    );
}
