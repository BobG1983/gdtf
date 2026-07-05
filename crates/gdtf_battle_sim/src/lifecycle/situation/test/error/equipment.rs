//! Missing weapon / armor registry-key aborts (GTW-257 / GTW-269).

use super::super::support::*;

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
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &melee, &armor, &stat_tuning, None),
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
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &melee, &armor, &stat_tuning, None),
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
