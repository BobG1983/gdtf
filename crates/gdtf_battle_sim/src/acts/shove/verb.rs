//! Pure shove outcome: blocked, moved, or fell.

use crate::{
    falls::{DropLanding, resolve_drop},
    ganger::{Direction, Position},
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
};

/// Result of resolving a shove into the next cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveOutcome {
    /// Destination blocked or invalid.
    Blocked,
    /// Target moved to dest on the same level.
    Moved {
        /// New cell/level.
        dest: CellLevel,
    },
    /// Target was pushed off unsupported ground and fell.
    Fell {
        /// Cell they were shoved into (start of fall).
        dest:    CellLevel,
        /// Where the fall lands.
        landing: DropLanding,
    },
}

fn step_cell(cell: Cell, direction: Direction) -> Cell {
    let (dx, dy) = match direction {
        Direction::North => (0, -1),
        Direction::NorthEast => (1, -1),
        Direction::East => (1, 0),
        Direction::SouthEast => (1, 1),
        Direction::South => (0, 1),
        Direction::SouthWest => (-1, 1),
        Direction::West => (-1, 0),
        Direction::NorthWest => (-1, -1),
    };
    Cell::new(cell.x + dx, cell.y + dy)
}

/// Resolve a shove from attacker into target using surface and occupancy.
#[must_use]
pub fn resolve_shove(
    attacker: Position,
    target: Position,
    target_entity: bevy::prelude::Entity,
    surface: &SurfaceGrid,
    occupancy: &OccupancyGrid,
) -> ShoveOutcome {
    let attacker_cell = attacker.cell();
    let target_cell = target.cell();
    let level = target.level();

    let Some(direction) = Direction::from_cells(attacker_cell, target_cell) else {
        return ShoveOutcome::Blocked;
    };
    let dest_cell = step_cell(target_cell, direction);
    let dest = CellLevel::new(dest_cell, level);

    if let Some(occupant) = occupancy.occupant(&dest)
        && occupant != target_entity
    {
        return ShoveOutcome::Blocked;
    }
    if *occupancy.is_blocked(&dest) {
        return ShoveOutcome::Blocked;
    }

    let supported = *level == 0 || surface.slab_state(&dest) == crate::surface::SlabState::Present;
    if supported {
        return ShoveOutcome::Moved { dest };
    }
    match resolve_drop(dest_cell, level, surface) {
        Some(landing) => ShoveOutcome::Fell { dest, landing },
        None => ShoveOutcome::Moved { dest },
    }
}
