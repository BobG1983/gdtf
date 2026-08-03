use super::support::*;

#[test]
fn no_messages_means_no_mutation() {
    let (mut app, shooter, target) = fire_scenario();
    let posture_actor = app
        .world_mut()
        .spawn((
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::North),
            Aiming::new(false),
            Tu::new(60),
        ))
        .id();
    let downed_actor = spawn_downed_actor(app.world_mut(), 20, 20, 1);
    let downed_target = spawn_downed_target(app.world_mut(), 21, 20, 1);
    let move_actor = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(30, 30), Level::new(0))),
            Tu::new(80),
            LifeState::Alive,
        ))
        .id();

    let snap = |app: &App| {
        (
            app.world().get::<Hp>(target).copied(),
            app.world().get::<Wounds>(target).copied(),
            app.world().get::<LifeState>(target).copied(),
            app.world().get::<Tu>(shooter).copied(),
            app.world().get::<Stance>(posture_actor).copied(),
            app.world().get::<Facing>(posture_actor).copied(),
            app.world().get::<Aiming>(posture_actor).copied(),
            app.world().get::<Tu>(posture_actor).copied(),
            app.world().get::<BleedingOut>(downed_target).copied(),
            app.world().get::<LifeState>(downed_target).copied(),
            app.world().get::<LifeState>(downed_actor).copied(),
            (
                app.world().get::<Position>(move_actor).copied(),
                app.world().get::<Tu>(move_actor).copied(),
            ),
        )
    };
    let before = snap(&app);

    for _ in 0..3 {
        app.update();
    }

    assert_eq!(
        snap(&app),
        before,
        "with no `*Requested` emitted every dispatch system is inert — nothing mutates",
    );
}

#[test]
fn dispatch_and_occupancy_co_schedule_and_a_kill_frees_the_slot() {
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
    let target = app.world_mut().spawn(target_bundle(1, 1)).id();
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    app.world_mut()
        .entity_mut(target)
        .insert(Position::new(target_at));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(target_at, Some(target));
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }
    if let Some(mut cover) = app.world_mut().get_resource_mut::<CoverLedger>() {
        cover.insert(
            target_at,
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::High,
                ArmorProtection::new(0),
                ArmorHardness::new(0),
            ),
        );
    }

    app.update();
    let occupied_before = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&target_at));
    assert_eq!(
        occupied_before,
        Some(target),
        "after initial sync the target occupies its slot",
    );

    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update(); 
    app.update(); 

    let life_after = app.world().get::<LifeState>(target).copied();
    let slot_after = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&target_at));
    assert!(
        !matches!(life_after, Some(LifeState::Alive)),
        "the fire must have downed/killed the fragile target, got {life_after:?}",
    );
    assert_eq!(
        slot_after, None,
        "the occupancy slot the dispatched kill vacated must be freed by the \
         co-scheduled occupancy system",
    );
}
