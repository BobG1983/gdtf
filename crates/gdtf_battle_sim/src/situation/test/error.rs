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
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &armor, &mut commands)
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
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &armor, &mut commands)
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
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &armor, &mut commands)
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
