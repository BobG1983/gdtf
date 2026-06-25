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
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored — `missing` dangles
        .vertical_link(link)
        .build();

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
                &registry,
                &armor,
                &stat_tuning,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                None,
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
    let situation = SituationBuilder::new().with_ganger(ganger).build();

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
                &registry,
                &armor,
                &stat_tuning,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                None,
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
    let situation = SituationBuilder::new().with_ganger(ganger).build();

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
                &registry,
                &armor,
                &stat_tuning,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                None,
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
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .with_scatter(CoverSpawn::new(key(2, 2, 0), missing.clone()))
        .build();

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
                &registry,
                &armor,
                &stat_tuning,
                Some(&terrain),
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

    let mut situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build();
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
                &registry,
                &armor,
                &stat_tuning,
                Some(&terrain),
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

    let mut situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build();
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
                &registry,
                &armor,
                &stat_tuning,
                Some(&terrain),
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
    let mut situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build();
    situation.default_floor = default_key;
    situation.floors = vec![FloorSpawn::new(overridden_cell, override_key)];

    let Some((mut app, _setup)) = run_setup_with(
        situation,
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
