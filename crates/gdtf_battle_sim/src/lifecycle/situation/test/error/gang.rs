//! GTW-414/415 — missing gang / missing member ref aborts.

use super::super::support::*;

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
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // No terrain authored in this fixture (the gang resolution fires first),
                // so terrain: None + the fallback floor cost suffices.
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
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
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
