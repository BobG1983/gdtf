//! GTW-234 AC3 (dispatch path) — a VALID `MoveRequested` dispatch moves the actor to the
//! dest AND drops its Tu by EXACTLY the destination terrain's looked-up move cost (a
//! relation to the tuning leaf, never a pinned magnitude). Plus GTW-234 AC7 — the move
//! dispatch co-schedules with `sync_moved_gangers` in `SimSystems::Simulate`.

use super::support::*;

/// Spawn a move-capable actor ([`Position`] / [`Tu`] / [`LifeState::Alive`] /
/// [`Faction`]) at `(x, y, 0)`.
///
/// GTW-354: the move dispatch fetches `&Faction` (to classify route occupants relative to
/// the mover) and runs `find_path` — so a move actor carries the player gang
/// ([`TEST_PLAYER_GANG`]) so it is found by the actor query and so its own-squad relation
/// resolves.
fn spawn_move_actor(world: &mut World, x: i32, y: i32, tu: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Tu::new(tu),
            LifeState::Alive,
            Faction::new(TEST_PLAYER_GANG),
        ))
        .id()
}

/// Drain the buffered [`MovementOccurred`] combat-log messages emitted this run (GTW-328).
fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

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

    // The looked-up cost the dispatch will charge — read off the SAME resources the
    // dispatch reads (the dest's terrain × the move_costs table), a relation never a
    // pinned magnitude.
    let expected_cost = app.world().get_resource::<CombatTuning>().and_then(|t| {
        app.world()
            .get_resource::<OccupancyGrid>()
            .map(|g| *t.move_costs.cost(g.terrain(&dest)))
    });
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
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // BOTH plugins tag into SimSystems::Simulate; OccupancyMaintenancePlugin owns the
    // set's configure_sets, SimActsPlugin only `.in_set`s into it.
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app);

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
