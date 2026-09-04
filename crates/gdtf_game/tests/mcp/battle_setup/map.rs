//! Cells picked out of the generated map, so a case names a real place rather than a literal.

use bevy::app::App;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    acts::downed::is_8_adjacent,
    ganger::Position,
    occupancy::TerrainKind,
    prelude::{CellLevel, OccupancyGrid},
    visibility::SquadVisibility,
};
use gdtf_game::qa_wire::cell::CellLevelNet;

use super::expected::Standing;

/// Frames a fixture runs to let the app's own systems finish, either side of its write.
const SETTLE_FRAMES: u8 = 8;

/// The fog the screen is drawing, which every battle read answers from.
pub(super) fn shown_fog(app: &App) -> Option<&SquadVisibility> {
    app.world()
        .get_resource::<ShownSquadVisibility>()
        .map(ShownSquadVisibility::visibility)
}

/// The first wall or cover cell the screen is lighting.
pub(crate) fn a_lit_cover_cell(app: &App) -> Option<CellLevelNet> {
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

/// The fog the sim itself reads, which is what `record_acts` resolves an act's witnesses from.
pub(super) fn live_fog(app: &App) -> Option<&SquadVisibility> {
    app.world().get_resource::<SquadVisibility>()
}

/// Three cells on one row: an unlit stand cell, a lit cell past it, and an unlit cell past that.
/// A round fired from the first towards the third crosses the second.
pub(super) fn a_row_across_the_lit_area(app: &App) -> Option<(CellLevel, CellLevel, CellLevel)> {
    let fog = live_fog(app)?;
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    map_cells(grid).into_iter().find_map(|lit| {
        if !*fog.is_cell_visible(&lit) {
            return None;
        }
        let (cell, level) = lit.split();
        let muzzle = first_dark_on_row(fog, grid, cell, level, -1, true)?;
        let impact = first_dark_on_row(fog, grid, cell, level, 1, false)?;
        Some((muzzle, lit, impact))
    })
}

/// The first unlit cell walking away from `from` along the row, optionally needing it free.
fn first_dark_on_row(
    fog: &SquadVisibility,
    grid: &OccupancyGrid,
    from: gdtf_battle_sim::prelude::Cell,
    level: gdtf_battle_sim::prelude::Level,
    step: i32,
    must_be_free: bool,
) -> Option<CellLevel> {
    (1..24).find_map(|distance| {
        let at = CellLevel::new(
            gdtf_battle_sim::prelude::Cell::new(from.x + step * distance, from.y),
            level,
        );
        if *fog.is_cell_visible(&at) {
            return None;
        }
        if must_be_free
            && !(grid.occupant(&at).is_none() && matches!(grid.terrain(&at), TerrainKind::Open))
        {
            return None;
        }
        Some(at)
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
