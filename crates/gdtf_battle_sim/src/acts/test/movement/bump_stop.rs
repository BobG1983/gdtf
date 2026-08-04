use super::support::*;

#[test]
fn walk_bump_stop_halts_on_a_tag_only_path_block_added_mid_walk() {
    use crate::terrain::entity::{BlocksPathfinding, TerrainCell};

    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

    let block_cell = CellLevel::new(Cell::new(13, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(14, 10), Level::new(0));

    let mover = spawn_move_actor(app.world_mut(), 10, 10, 100);

    app.update();
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    app.update();

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

    app.world_mut()
        .spawn((TerrainCell::new(block_cell), BlocksPathfinding));

    for _ in 0..10 {
        app.update();
        let here = app.world().get::<Position>(mover).copied();
        assert_ne!(
            here,
            Some(Position::new(block_cell)),
            "the bump-stop must HALT before the tag-blocked cell (13,10) — it must NOT step \
             onto a cell the planner treats as impassable ",
        );
        assert_ne!(
            here,
            Some(Position::new(dest)),
            "the bump-stop must NOT let the mover walk THROUGH the tag-blocked cell to the \
             destination (14,10) — that is the kind-based-bump-stop bug D2 fixes",
        );
    }

    assert_eq!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .map(|g| (*g.is_blocked(&block_cell), *g.is_path_blocked(&block_cell))),
        Some((false, true)),
        "(13,10) blocks the PATH (tag) but not the kind surface — the tag/kind split (D1)",
    );
    let last_free = CellLevel::new(Cell::new(12, 10), Level::new(0));
    assert_eq!(
        app.world().get::<Position>(mover).copied(),
        Some(Position::new(last_free)),
        "the mover halts at the last free step (12,10) — one cell short of the tag block (13,10)",
    );
}
