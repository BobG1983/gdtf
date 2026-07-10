//! GTW-501 D2 — the tag-derived path-block bump-stop halting a walk mid-flight.

use super::support::*;

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
        .map(|g| *g.is_blocked(&block_cell));
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
            .map(|g| (*g.is_blocked(&block_cell), *g.is_path_blocked(&block_cell))),
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
