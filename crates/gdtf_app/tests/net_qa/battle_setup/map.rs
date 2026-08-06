//! Cells picked out of the generated map, so a case names a real place rather than a literal.

use bevy::app::App;
use gdtf_app::qa_wire::cell::CellLevelNet;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::Position,
    occupancy::TerrainKind,
    prelude::{CellLevel, OccupancyGrid},
    visibility::SquadVisibility,
};

use super::expected::Standing;

/// Frames a fixture runs to let the app's own systems finish, either side of its write.
const SETTLE_FRAMES: u8 = 8;

/// The fog the screen is drawing, which every battle read answers from.
pub(super) fn shown_fog(app: &App) -> Option<&SquadVisibility> {
    app.world()
        .get_resource::<ShownSquadVisibility>()
        .map(ShownSquadVisibility::visibility)
}

/// The first wall or cover cell of the generated map, which the inspect panel draws a block for.
pub(crate) fn a_cover_cell(app: &App) -> Option<CellLevelNet> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    map_cells(grid)
        .into_iter()
        .find(|at| {
            grid.occupant(at).is_none()
                && matches!(grid.terrain(at), TerrainKind::Wall | TerrainKind::Cover)
        })
        .map(CellLevelNet::from_sim)
}

/// The first wall or cover cell the screen is lighting.
pub(super) fn a_lit_cover_cell(app: &App) -> Option<CellLevelNet> {
    let fog = shown_fog(app)?;
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    map_cells(grid)
        .into_iter()
        .find(|at| {
            *fog.is_cell_visible(at)
                && matches!(grid.terrain(at), TerrainKind::Wall | TerrainKind::Cover)
        })
        .map(CellLevelNet::from_sim)
}

/// The first empty floor cell standing inside or outside the area the screen lights.
pub(super) fn a_free_open_cell(app: &App, standing: Standing) -> Option<CellLevel> {
    let fog = shown_fog(app)?;
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    map_cells(grid).into_iter().find(|at| {
        let lit = *fog.is_cell_visible(at);
        let wanted = match standing {
            Standing::Lit => lit,
            Standing::Hidden => !lit,
        };
        wanted && grid.occupant(at).is_none() && matches!(grid.terrain(at), TerrainKind::Open)
    })
}

/// The first empty floor cell 8-adjacent to `beside` and 8-adjacent to nothing in `clear_of`.
pub(super) fn a_free_open_cell_beside(
    app: &App,
    beside: CellLevel,
    clear_of: &[CellLevel],
) -> Option<CellLevel> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    map_cells(grid).into_iter().find(|at| {
        *is_8_adjacent(Position::new(beside), Position::new(*at))
            && clear_of
                .iter()
                .all(|away| !*is_8_adjacent(Position::new(*away), Position::new(*at)))
            && grid.occupant(at).is_none()
            && matches!(grid.terrain(at), TerrainKind::Open)
    })
}

/// Run the app far enough that the systems writing around this point have finished.
pub(super) fn settle(app: &mut App) {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
}

fn map_cells(grid: &OccupancyGrid) -> Vec<CellLevel> {
    let mut cells: Vec<CellLevel> = grid.all_cells().collect();
    cells.sort_unstable_by_key(|at| (at.level(), at.cell().y, at.cell().x));
    cells
}
