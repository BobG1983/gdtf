use super::support::*;

#[test]
fn move_dispatch_emits_one_movement_occurred_with_from_and_to() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); 

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let movements = drain_movements(&mut app);
    assert_eq!(
        movements.len(),
        1,
        "a valid move emits exactly one MovementOccurred: {movements:?}",
    );
    let Some(moved) = movements.first() else {
        return;
    };
    assert_eq!(moved.actor, actor, "the signal carries the moving actor");
    assert_eq!(
        moved.from,
        Cell::new(10, 10),
        "the FROM cell is the actor's pre-move ground cell",
    );
    assert_eq!(
        moved.to,
        Cell::new(11, 10),
        "the TO cell is the actor's post-move ground cell (the destination)",
    );
}

#[test]
fn blocked_move_emits_no_movement_occurred() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    let blocker = spawn_move_actor(app.world_mut(), 11, 10, 100);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(dest, Some(blocker));
    }

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_movements(&mut app).is_empty(),
        "a blocked (occupied-dest) move is a no-op — it announces no MovementOccurred",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(CellLevel::new(
            Cell::new(10, 10),
            Level::new(0)
        ))),
        "a blocked move leaves the actor in place",
    );
}

#[test]
fn move_dispatch_steps_the_actor_and_spends_the_dest_terrain_cost() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); 

    let expected_cost = app
        .world()
        .get_resource::<FloorCostGrid>()
        .map(|fc| *fc.cost(&dest));
    let tu_before = app.world().get::<Tu>(actor).map(|t| **t);

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "move dispatch must step the actor to the requested destination",
    );
    let tu_after = app.world().get::<Tu>(actor).map(|t| **t);
    assert!(
        matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b),
        "a real move must strictly decrease Tu",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        expected_cost,
        "the Tu drop must equal exactly the destination terrain's looked-up move cost",
    );
}

#[test]
fn move_dispatch_and_occupancy_co_schedule_fills_dest_and_frees_source() {
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

    let source = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); 
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);

    app.update();
    let source_occupied = app
        .world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&source));
    assert_eq!(
        source_occupied,
        Some(actor),
        "after initial sync the actor occupies its source slot",
    );

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update(); 
    app.update(); 

    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "the dispatched move wrote Position=dest",
    );
    let grid = app.world().get_resource::<OccupancyGrid>();
    assert_eq!(
        grid.and_then(|g| g.occupant(&dest)),
        Some(actor),
        "sync_moved_gangers (co-scheduled) sets the dest slot occupant",
    );
    assert_eq!(
        grid.and_then(|g| g.occupant(&source)),
        None,
        "sync_moved_gangers (co-scheduled) frees the source slot",
    );
}
