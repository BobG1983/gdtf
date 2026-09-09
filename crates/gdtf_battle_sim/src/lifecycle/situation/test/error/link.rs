use super::super::support::*;

/// A bad authored vertical link (dangling endpoint) makes `setup_battle` return
#[test]
fn setup_aborts_on_invalid_vertical_link() {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1);
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let (situation, placements, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present)
        .vertical_link(link)
        .build_with_gangs();

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
        Some(BattleSetupError::InvalidLink(
            InvalidVerticalLink::DanglingCell { link },
        )),
        "an invalid vertical link must abort setup with the typed error",
    );

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
