use super::support::*;
use crate::{occupancy::OccupancyGrid, occupancy_sync::TerrainPieceDestroyed};

fn path_blocked_at(app: &bevy::app::App, at: crate::metric::CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|grid| *grid.is_path_blocked(&at))
}

#[test]
fn a_piece_that_leaves_nothing_behind_stops_blocking_the_path() {
    let at = key(11, 6, 0);
    let mut app = battle_with(&[(at, PLAIN_COVER)]);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    assert_eq!(
        path_blocked_at(&app, at),
        Some(false),
        "the destroyed piece lost BlocksPathfinding, and project_path_blocking cleared the cell",
    );
}

#[test]
fn a_blocking_successor_keeps_the_cell_path_blocked() {
    let at = key(12, 6, 0);
    let mut app = battle_with(&[(at, SMASHED_COVER)]);

    app.world_mut()
        .write_message(TerrainPieceDestroyed::new(at, COVER_KIND));
    app.update();

    assert_eq!(
        path_blocked_at(&app, at),
        Some(true),
        "the successor blocks pathing, and blocking is read from the piece standing in the cell \
         rather than from a destroyed-cover mark",
    );
}
