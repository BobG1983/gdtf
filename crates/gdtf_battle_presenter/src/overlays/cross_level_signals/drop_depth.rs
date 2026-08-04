use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    falls::{StoreysFallen, resolve_drop},
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    visibility::SquadVisibility,
};

const ORTHOGONAL_OFFSETS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub(super) fn gather_drop_depth(
    active_level: Level,
    surface: &SurfaceGrid,
    occupancy: &OccupancyGrid,
    drawn_on_active: &HashSet<Cell>,
    squad: &SquadVisibility,
) -> Vec<(Cell, StoreysFallen)> {
    if *active_level == 0 {
        return Vec::new();
    }

    let mut candidates: HashSet<Cell> = HashSet::default();
    for &cell in drawn_on_active {
        candidates.insert(cell);
        for (dx, dy) in ORTHOGONAL_OFFSETS {
            candidates.insert(Cell::new(cell.x + dx, cell.y + dy));
        }
    }

    let mut out = Vec::new();
    for cell in candidates {
        let key = CellLevel::new(cell, active_level);
        if !*squad.is_cell_explored(&key) {
            continue;
        }
        if occupancy.terrain(&key) != TerrainKind::Open {
            continue;
        }
        if surface.slab_state(&key) == SlabState::Present {
            continue;
        }
        if let Some(landing) = resolve_drop(cell, active_level, surface) {
            out.push((cell, landing.storeys));
        }
    }
    out
}
