//! Drop-depth gathering (GTW-596): the "hole / ledge cell" candidate scan + the
//! real [`resolve_drop`] fall-distance read.
//!
//! A hole/ledge candidate is an OPEN, unsupported cell (no `Present` slab) that
//! borders the currently-drawn terrain on the active storey — bounding the scan
//! to the map's built footprint rather than the whole 60×60 grid void. There is
//! no sim API enumerating "the floor footprint of storey N" directly; the drawn
//! `TerrainSprite` set + its immediate 4-neighbourhood is this presenter's own
//! bounded proxy for it (a destroyed slab still carries a drawn `TerrainSprite`
//! — the swap reaction retargets its material rather than despawning it — so it
//! is captured directly; a permanently-authored ledge gap is captured via its
//! drawn neighbour).

use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    falls::{StoreysFallen, resolve_drop},
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    visibility::SquadVisibility,
};

/// The four orthogonal neighbour offsets — a presenter-local BOUNDING helper only
/// (NOT the sim's 8-connected `pathable_neighbors` adjacency rule): it widens the
/// drop-depth scan from "drawn terrain cells" to their immediate border, nothing
/// gameplay-authoritative.
const ORTHOGONAL_OFFSETS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Gather one `(cell, storeys-fallen)` entry per fog-EXPLORED hole/ledge cell on
/// `active_level` — a walkable ([`TerrainKind::Open`]) cell with no supporting
/// slab, bordering the drawn terrain footprint.
///
/// Ground (`active_level == 0`) never drops — [`resolve_drop`]'s own guard — so
/// this returns empty immediately rather than scanning.
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
            continue; // never surface a terrain fact the squad hasn't seen
        }
        if occupancy.terrain(&key) != TerrainKind::Open {
            continue; // a wall / cover cell is not a walkable hole
        }
        if surface.slab_state(&key) == SlabState::Present {
            continue; // supported — not a hole
        }
        if let Some(landing) = resolve_drop(cell, active_level, surface) {
            out.push((cell, landing.storeys));
        }
    }
    out
}
