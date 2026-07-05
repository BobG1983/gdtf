//! GTW-234/328 — the valid-move dispatch: cost relation, the `MovementOccurred`
//! signal, the blocked no-op, and the occupancy co-schedule.

use super::support::*;

// GTW-328 — a VALID move emits exactly one MovementOccurred carrying the actor and the
// actual FROM (pre-write) and TO (post-write) ground cells.
#[test]
fn move_dispatch_emits_one_movement_occurred_with_from_and_to() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Open, empty, in-bounds

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

// GTW-328 — a BLOCKED move (the destination is occupied) is a total no-op and emits NO
// MovementOccurred: the log only announces real steps. A second actor sits on the dest.
#[test]
fn blocked_move_emits_no_movement_occurred() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    // Park a blocker on the destination slot so the move's occupancy gate fails.
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
    // And the actor did NOT move (the no-op contract).
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
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Open, empty, in-bounds

    // The looked-up cost the dispatch will charge — read off the SAME resource the
    // dispatch reads (the FloorCostGrid at the destination cell, GTW-396 Decision C1),
    // a relation never a pinned magnitude.
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
    // The canonical acts harness (SimActsPlugin + litany + full vision) plus the live
    // maintenance layer: OccupancyMaintenancePlugin owns SimSystems::Simulate's
    // configure_sets, SimActsPlugin only `.in_set`s into it.
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

    let source = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0)); // Open, empty, in-bounds
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);

    // First update: sync_moved_gangers reacts to the actor's Changed<Position>
    // (initial placement), marking the source slot. The combined schedule builds +
    // runs with no ambiguity/access panic (AC7's co-schedule check).
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

    // Now MOVE — dispatch_move writes Position=dest this update; sync_moved_gangers
    // reacts to the Changed<Position> next update, marking the dest and freeing source.
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update(); // dispatch_move writes Position=dest this update
    app.update(); // sync_moved_gangers reacts to Changed<Position> next update

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
