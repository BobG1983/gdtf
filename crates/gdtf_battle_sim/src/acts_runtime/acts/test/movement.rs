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
            let at = CellLevel::new(Cell::new(x, y), Level::new(0));
            grid.set_terrain(at, TerrainKind::Wall);
            // GTW-501: find_path reads the TAG-derived path-blocking surface, so a hand-set
            // wall must also mark that surface (mirroring the projection a real spawned wall
            // entity's BlocksPathfinding marker yields).
            grid.set_path_blocking(at);
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

// GTW-501 D2 / C3 — the runtime bump-stop reads the TAG-DERIVED path-blocking surface
// (`is_path_blocked`), NOT the kind-based `is_blocked`. The discriminating scenario the
// kind-based bump-stop would FAIL: a route cell that is OPEN (so kind-based
// `is_blocked == false`) gains a `BlocksPathfinding` marker mid-walk — the C3 capability
// the projection supports. With the bump-stop reading `is_path_blocked`, the mover must
// HALT before that cell (planner and executor in lock-step); the old kind-based bump-stop
// would walk THROUGH it (it sees the cell as Open). The marker is added via the REAL
// projection pipeline (a spawned terrain entity + `project_path_blocking`), end to end.
#[test]
fn walk_bump_stop_halts_on_a_tag_only_path_block_added_mid_walk() {
    use crate::terrain::entity::{BlocksPathfinding, TerrainCell};

    let mut app = headless_app();
    // The live maintenance layer owns the SimSystems::Simulate set + project_path_blocking;
    // SimActsPlugin only `.in_set`s `advance_walk` into it (ordered .after(project_path_blocking)).
    app.add_plugins(OccupancyMaintenancePlugin);
    insert_sim_resources(&mut app);

    // A straight, fully-OPEN east route (10,10)->(14,10): every route cell is Open, so the
    // kind-based `is_blocked` is FALSE on ALL of them — the bump-stop reading kind would
    // never halt here. The cell we will block mid-walk is (13,10).
    let block_cell = CellLevel::new(Cell::new(13, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(14, 10), Level::new(0));

    let mover = spawn_move_actor(app.world_mut(), 10, 10, 100);

    // Tick once so initial placement publishes the occupant slot, then START the walk.
    app.update();
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    app.update(); // dispatch_move plans the route + attaches WalkInProgress; first step lands.

    // The mover has begun walking but has NOT yet reached the cell we will block. Sanity:
    // (13,10) is genuinely OPEN on the kind-based surface — proving the halt below is the
    // TAG surface doing the work, not a kind block.
    let kind_blocks_before = app
        .world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_blocked(&block_cell));
    assert_eq!(
        kind_blocks_before,
        Some(false),
        "precondition: (13,10) is Open on the kind-based surface (is_blocked == false), so \
         only the tag surface can halt the mover there",
    );
    assert_ne!(
        app.world().get::<Position>(mover).copied(),
        Some(Position::new(dest)),
        "precondition: the mover has not yet reached the destination — the walk is mid-flight",
    );

    // C3: add the BlocksPathfinding marker onto (13,10) at RUNTIME — via the REAL pipeline
    // (a spawned terrain entity carrying the marker). The terrain kind stays OPEN, so this
    // is a TAG-ONLY block: is_path_blocked(13,10) becomes true while is_blocked stays false.
    app.world_mut()
        .spawn((TerrainCell::new(block_cell), BlocksPathfinding));

    // Drive the walk to settle. project_path_blocking folds the new marker into the surface
    // BEFORE advance_walk's bump-stop reads it the same tick (the .after ordering, C3). Assert
    // on EVERY tick that the mover never steps onto OR past the tag-blocked cell.
    for _ in 0..10 {
        app.update();
        let here = app.world().get::<Position>(mover).copied();
        assert_ne!(
            here,
            Some(Position::new(block_cell)),
            "the bump-stop must HALT before the tag-blocked cell (13,10) — it must NOT step \
             onto a cell the planner treats as impassable (GTW-501 D2)",
        );
        assert_ne!(
            here,
            Some(Position::new(dest)),
            "the bump-stop must NOT let the mover walk THROUGH the tag-blocked cell to the \
             destination (14,10) — that is the kind-based-bump-stop bug GTW-501 D2 fixes",
        );
    }

    // The kind surface is STILL Open at (13,10) (C4 / D1: tags drive PATH only, kind unchanged)
    // — the halt was purely the tag-derived surface, the whole point of the ticket.
    assert_eq!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .map(|g| (g.is_blocked(&block_cell), g.is_path_blocked(&block_cell))),
        Some((false, true)),
        "(13,10) blocks the PATH (tag) but not the kind surface — the tag/kind split (D1)",
    );
    // And the mover halts at the LAST FREE step (12,10) — one cell short of the tag block
    // (13,10) — proving it stopped at the bump-stop rather than teleporting through.
    let last_free = CellLevel::new(Cell::new(12, 10), Level::new(0));
    assert_eq!(
        app.world().get::<Position>(mover).copied(),
        Some(Position::new(last_free)),
        "the mover halts at the last free step (12,10) — one cell short of the tag block (13,10)",
    );
}
