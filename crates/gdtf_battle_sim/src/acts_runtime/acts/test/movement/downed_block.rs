//! GTW-459 regression — the planner refuses to route through a downed body.

use super::support::*;

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

    // The canonical acts harness (SimActsPlugin + litany + full vision) plus the live
    // maintenance layer (owns the SimSystems::Simulate set + sync_dead_gangers, the
    // system under test); OccupancyMaintenancePlugin owns its configure_sets.
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

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
