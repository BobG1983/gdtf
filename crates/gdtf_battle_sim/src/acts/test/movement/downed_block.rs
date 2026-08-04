use super::support::*;

#[test]
fn enemy_never_routes_through_a_downed_friendly() {
    use crate::occupancy::TerrainKind;

    const ENEMY_GANG: u8 = 9; 

    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);

    let downed_cell = CellLevel::new(Cell::new(11, 10), Level::new(0));
    let dest = CellLevel::new(Cell::new(12, 10), Level::new(0));

    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        let walls = [
            (11, 9),
            (11, 11),
            (12, 9),
            (12, 11),
            (13, 9),
            (13, 10),
            (13, 11),
            (10, 9),
            (10, 11),
        ];
        for (x, y) in walls {
            let at = CellLevel::new(Cell::new(x, y), Level::new(0));
            grid.set_terrain(at, TerrainKind::Wall);
            grid.set_path_blocking(at);
        }
    }

    let friendly = spawn_move_actor_of_gang(app.world_mut(), 11, 10, 0, TEST_PLAYER_GANG);
    let enemy = spawn_move_actor_of_gang(app.world_mut(), 10, 10, 100, ENEMY_GANG);

    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(friendly) {
        *life = LifeState::Downed;
    }
    app.update();
    assert_eq!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&downed_cell)),
        Some(friendly),
        "the downed friendly must STILL occupy its cell — the precondition \
         for this regression test",
    );

    app.world_mut()
        .write_message(MoveRequested::new(enemy, dest));

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
              — it routed through / onto the downed body",
        );
    }

    assert!(
        saw_unreachable,
        "the planner must refuse to route through the downed body — a \
         MoveRejection::Unreachable is expected; had Downed freed its cell, \
         find_path would have routed (10,10)->(11,10)->(12,10) instead",
    );
    assert_eq!(
        app.world().get::<Position>(enemy).copied(),
        Some(Position::new(CellLevel::new(
            Cell::new(10, 10),
            Level::new(0)
        ))),
        "the enemy stays at its start cell — no route through the downed body ",
    );
}
