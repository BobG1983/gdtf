//! GTW-234 AC3 (dispatch path) — a VALID `MoveRequested` dispatch moves the actor to the
//! dest AND drops its Tu by EXACTLY the destination terrain's looked-up move cost (a
//! relation to the tuning leaf, never a pinned magnitude). Plus GTW-234 AC7 — the move
//! dispatch co-schedules with `sync_moved_gangers` in `SimSystems::Simulate`.

use super::support::*;

/// Spawn a move-capable actor ([`Position`] / [`Tu`] / [`LifeState::Alive`]) at
/// `(x, y, 0)`.
fn spawn_move_actor(world: &mut World, x: i32, y: i32, tu: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Tu::new(tu),
            LifeState::Alive,
        ))
        .id()
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
