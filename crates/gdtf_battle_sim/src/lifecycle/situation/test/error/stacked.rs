use super::super::support::*;

/// — TWO authored gangers on the SAME `(cell, level)` make `setup_battle`
#[test]
fn setup_errors_on_stacked_gangers() {
    let shared = key(5, 5, 0);
    let (situation, gangs) = SituationBuilder::new()
        .with_gangers([ganger_at(shared, 0), ganger_at(shared, 1)])
        .build_with_gangs();
    assert_eq!(situation.gangers.len(), 2, "the fixture fields two gangers");
    assert!(
        situation.gangers.iter().all(|g| g.at == shared),
        "both gangers are authored on the one shared cell",
    );

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
        Some(BattleSetupError::StackedGangers { at: shared }),
        "two gangers on one cell must abort setup with StackedGangers, naming the cell",
    );

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
