use super::super::support::*;

#[test]
fn setup_errors_on_a_missing_gang_key() {
    let (situation, mut placements, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    let missing_gang = GangName::new("no-such-gang".to_owned());
    assert!(
        !placements.is_empty(),
        "the fixture must field a ganger to re-point",
    );
    for placed in &mut placements {
        placed.gang = missing_gang.clone();
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation.map,
                &placements,
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

#[test]
fn setup_errors_on_a_missing_member_key() {
    let (situation, mut placements, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    let present_gang = GangName::new("gang_0".to_owned());
    let missing_member = GangerName::new("no-such-member".to_owned());
    assert!(
        gangs.roster(&present_gang).is_some(),
        "the synthesized registry must hold gang_0 (so the gang resolves, isolating the member)",
    );
    assert!(
        !placements.is_empty(),
        "the fixture must field a ganger to re-point",
    );
    for placed in &mut placements {
        placed.gang = present_gang.clone();
        placed.member = missing_member.clone();
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation.map,
                &placements,
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
