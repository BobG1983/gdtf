//! A destroyed emplacement puts its occupant back on the ground.

use bevy::{
    ecs::{query::QueryData, relationship::Relationship},
    prelude::{Commands, Deref, Entity, MessageReader, Query, Res},
};

use super::{
    clear_seat,
    relationship::MountedBy,
    state::{EmplacementState, EnteredFrom, MountedWeaponEntity},
};
use crate::{
    ganger::Position,
    metric::{Cell, CellLevel},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    occupancy_sync::TerrainPieceDestroyed,
    terrain::entity::{TerrainCell, TerrainPieceKind},
};

// Whether a cell can take the ejected occupant.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct LandingFree(bool);

impl LandingFree {
    // Wrap a free flag.
    const fn new(free: bool) -> Self {
        Self(free)
    }
}

// How far out from the seat a scan step reaches, in Chebyshev distance.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct ScanDistance(i32);

impl ScanDistance {
    // Wrap a distance in cells.
    const fn new(cells: i32) -> Self {
        Self(cells)
    }

    // The last distance that can still reach a grid cell from anywhere inside the grid.
    fn grid_span() -> Self {
        let span = if GRID_WIDTH > GRID_HEIGHT {
            GRID_WIDTH
        } else {
            GRID_HEIGHT
        };
        Self(i32::try_from(span).unwrap_or(i32::MAX))
    }
}

/// One emplacement's columns: where it stands, its state, its occupant and its mount.
#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct SeatRow {
    /// The emplacement entity.
    pub(crate) entity:   Entity,
    /// Cell the piece stands on.
    pub(crate) cell:     &'static TerrainCell,
    /// Vacant or occupied.
    pub(crate) state:    &'static mut EmplacementState,
    /// The ganger manning it.
    pub(crate) occupant: Option<&'static MountedBy>,
    /// The cell that ganger entered from.
    pub(crate) entered:  Option<&'static EnteredFrom>,
    /// The weapon bolted to it.
    pub(crate) mount:    Option<&'static MountedWeaponEntity>,
}

/// Put the occupant of every destroyed emplacement back on the ground and free its seat.
pub(crate) fn eject_on_destroy(
    mut commands: Commands,
    mut destroyed: MessageReader<TerrainPieceDestroyed>,
    mut seats: Query<SeatRow>,
    mut occupants: Query<&mut Position>,
    grid: Res<OccupancyGrid>,
) {
    for event in destroyed.read() {
        if event.kind != TerrainPieceKind::Emplacement {
            continue;
        }
        for mut seat in &mut seats {
            if **seat.cell != event.at || !*seat.state.is_occupied() {
                continue;
            }
            let Some(occupant) = seat.occupant.map(Relationship::get) else {
                continue;
            };
            let entered = seat.entered.map(|from| **from);
            let landing = landing_cell(**seat.cell, entered, occupant, &grid);
            if let (Some(landing), Ok(mut position)) = (landing, occupants.get_mut(occupant)) {
                *position = Position::new(landing);
            }
            clear_seat(&mut commands, seat.entity, &mut seat.state, seat.mount);
        }
    }
}

// The cell it entered from when that is free, else the first free cell the scan reaches.
fn landing_cell(
    seat: CellLevel,
    entered: Option<CellLevel>,
    occupant: Entity,
    grid: &OccupancyGrid,
) -> Option<CellLevel> {
    if let Some(from) = entered
        && *cell_is_free(&from, occupant, grid)
    {
        return Some(from);
    }
    nearest_free_cell(seat, occupant, grid)
}

// Free is not path-blocked and unoccupied; the occupant itself does not take a cell.
fn cell_is_free(at: &CellLevel, occupant: Entity, grid: &OccupancyGrid) -> LandingFree {
    let held = grid
        .occupant(at)
        .is_some_and(|standing| standing != occupant);
    LandingFree::new(!*grid.is_path_blocked(at) && !held)
}

// Scan outward by Chebyshev distance, starting on the seat, until a ring leaves the grid.
fn nearest_free_cell(seat: CellLevel, occupant: Entity, grid: &OccupancyGrid) -> Option<CellLevel> {
    for step in 0..=*ScanDistance::grid_span() {
        let ring = ring_cells(seat, ScanDistance::new(step));
        if ring.iter().all(|cell| grid.slot(cell).is_none()) {
            return None;
        }
        let free = ring
            .into_iter()
            .find(|cell| grid.slot(cell).is_some() && *cell_is_free(cell, occupant, grid));
        if free.is_some() {
            return free;
        }
    }
    None
}

// Every cell exactly this far from the seat, on the seat's level, in ascending (y, x).
fn ring_cells(seat: CellLevel, distance: ScanDistance) -> Vec<CellLevel> {
    let level = seat.level();
    let span = *distance;
    let mut cells = Vec::new();
    for dy in -span..=span {
        for dx in -span..=span {
            if dx.abs().max(dy.abs()) != span {
                continue;
            }
            cells.push(CellLevel::new(Cell::new(seat.x + dx, seat.y + dy), level));
        }
    }
    cells
}
