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

/// Spawn a move-capable actor of `gang` (its own faction) at `(x, y, 0)` — the
/// faction-parameterized sibling of [`spawn_move_actor`] (which always uses
/// [`TEST_PLAYER_GANG`]). Used by the GTW-459 regression test to put an ENEMY mover
/// (a non-player gang) on the field alongside a player-gang downed body.
fn spawn_move_actor_of_gang(world: &mut World, x: i32, y: i32, tu: u8, gang: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Tu::new(tu),
            LifeState::Alive,
            Faction::new(gang),
        ))
        .id()
}

/// Drain the buffered [`MoveRejected`] signals emitted this run (GTW-354).
fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

/// GTW-459 (the regression test) — an ENEMY driven toward a DOWNED friendly via the
/// REAL movement path (`dispatch_move` → `find_path` → `advance_walk`) with the live
/// [`OccupancyMaintenancePlugin`] wired NEVER routes onto / through the downed body,
/// and NEVER writes a `Position` onto its cell.
///
/// Arena (a 1-wide corridor along `y = 10`, walls boxing it so the ONLY route from
/// the enemy's cell to the destination passes THROUGH the downed cell):
/// ```text
///   x:   9    10    11    12    13
///  y=9: WALL  WALL  WALL  WALL  WALL
///  y=10: .    ENEMY DOWN  DEST  .
///  y=11: WALL  WALL  WALL  WALL  WALL
/// ```
/// The player-gang friendly at `(11,10)` is downed BEFORE the enemy's move; under the
/// GTW-459 fix it HOLDS its occupant slot, so `find_path` cannot reach `(12,10)` (the
/// sole gateway is occupied) and returns [`MoveRejection::Unreachable`] — no walk
/// starts, the enemy stays put. The walk bump-stop (which refuses ANY occupied next
/// cell) is the second guard; with the planner already refusing, the enemy never even
/// approaches.
///
/// Pin-discrimination (C4): were C1 reverted (Downed freeing its cell), `(11,10)` would
/// read empty, `find_path` WOULD route `(10,10) → (11,10) → (12,10)`, and `advance_walk`
/// would step the enemy ONTO `(11,10)` — co-locating with the downed body. This test
/// would then fail on BOTH the no-co-location assert AND the Unreachable-reject assert.
/// Deterministic: seeded RNG (the harness `SEED`), tick-driven, no wall-clock.
#[test]
fn enemy_never_routes_through_a_downed_friendly() {
    use crate::occupancy::TerrainKind;

    const ENEMY_GANG: u8 = 9; // any gang != TEST_PLAYER_GANG (the player/friendly gang)

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The live maintenance layer (owns the SimSystems::Simulate set + sync_dead_gangers,
    // the system under test) + the act dispatch (dispatch_move / advance_walk). Both tag
    // into SimSystems::Simulate; OccupancyMaintenancePlugin owns its configure_sets.
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app);

    let downed_cell = CellLevel::new(Cell::new(11, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(12, 10), Level::new(0));

    // Seal `dest` (12,10) inside a fully-walled pocket whose ONLY open entrance is its
    // west neighbour `downed_cell` (11,10). Every other neighbour of `dest` AND of
    // `downed_cell` (except the enemy's approach at (10,10)) is a Wall, so the only route
    // from the enemy into the pocket runs THROUGH the downed body. The pocket's wall ring
    // (the 8 cells around `dest`, minus the gateway) plus the cells sealing the gateway's
    // own flanks (above/below (11,10)):
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        let walls = [
            // Ring around the pocket cell (12,10), minus the (11,10) gateway:
            (11, 9),
            (11, 11),
            (12, 9),
            (12, 11),
            (13, 9),
            (13, 10),
            (13, 11),
            // Seal the gateway's own flanks so the enemy cannot slip diagonally past it:
            (10, 9),
            (10, 11),
        ];
        for (x, y) in walls {
            grid.set_terrain(
                CellLevel::new(Cell::new(x, y), Level::new(0)),
                TerrainKind::Wall,
            );
        }
    }

    // The player-gang FRIENDLY body at (11,10), and the ENEMY mover at (10,10).
    let friendly = spawn_move_actor_of_gang(app.world_mut(), 11, 10, 0, TEST_PLAYER_GANG);
    let enemy = spawn_move_actor_of_gang(app.world_mut(), 10, 10, 100, ENEMY_GANG);

    // Tick once so initial placement publishes both occupant slots (Changed<Position>).
    app.update();

    // Down the FRIENDLY — under GTW-459 it RETAINS its (11,10) slot.
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(friendly) {
        *life = LifeState::Downed;
    }
    app.update();
    assert_eq!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&downed_cell)),
        Some(friendly),
        "the downed friendly must STILL occupy its cell (GTW-459 C1) — the precondition \
         for this regression test",
    );

    // Drive the enemy's move toward the cell BEYOND the downed body.
    app.world_mut()
        .write_message(MoveRequested::new(enemy, dest));

    // Drive the full (multi-tick) walk to settle, asserting on EVERY tick that the
    // enemy never co-locates with the downed body.
    let mut saw_unreachable = false;
    for _ in 0..12 {
        app.update();
        for reject in drain_rejects(&mut app) {
            if reject.actor == enemy && reject.reason == MoveRejection::Unreachable {
                saw_unreachable = true;
            }
        }
        assert_ne!(
            app.world().get::<Position>(enemy).copied(),
            Some(Position::new(downed_cell)),
            "the enemy must NEVER write a Position onto the downed friendly's cell \
             (GTW-459 C4) — it routed through / onto the downed body",
        );
    }

    // The planner refused to route through the downed body (the sole gateway), so the
    // move was rejected Unreachable and the enemy never moved off its start.
    assert!(
        saw_unreachable,
        "the planner must refuse to route through the downed body — a \
         MoveRejection::Unreachable is expected (GTW-459 C4); had Downed freed its cell, \
         find_path would have routed (10,10)->(11,10)->(12,10) instead",
    );
    assert_eq!(
        app.world().get::<Position>(enemy).copied(),
        Some(Position::new(CellLevel::new(
            Cell::new(10, 10),
            Level::new(0)
        ))),
        "the enemy stays at its start cell — no route through the downed body (GTW-459)",
    );
}
