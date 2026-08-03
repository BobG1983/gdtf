use super::super::support::*;

#[test]
fn setup_errors_on_a_missing_terrain_key() {
    let missing = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_dead_0000_0001));
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .with_scatter(CoverSpawn::new(key(2, 2, 0), missing))
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let melee = test_melee_weapon_registry();
    let terrain = test_terrain_registry();
    let result = app
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
