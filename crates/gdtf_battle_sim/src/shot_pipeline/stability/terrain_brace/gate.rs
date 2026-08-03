//! Gate that decides whether the shooter is terrain-braced.

use bevy::prelude::Deref;

use crate::{
    ganger::{Position, StanceKind},
    metric::{CellLevel, Level, MAX_LEVELS},
    slab::BraceStairCells,
    surface::{SlabState, SurfaceGrid},
};

/// True when the shooter is crouching on a brace stair cell under a present slab.
#[must_use]
pub fn terrain_braces(
    position: Position,
    stance: StanceKind,
    brace_cells: &BraceStairCells,
    surface: &SurfaceGrid,
) -> TerrainBraced {
    if stance != StanceKind::Crouching {
        return TerrainBraced::new(false);
    }
    let cell = *position;
    if !brace_cells.contains(&cell) {
        return TerrainBraced::new(false);
    }
    let Some(above) = cell_above(cell) else {
        return TerrainBraced::new(false);
    };
    TerrainBraced::new(matches!(surface.slab_state(&above), SlabState::Present))
}

/// Cell one level above, or None at the top of the map.
#[must_use]
pub(crate) fn cell_above(cell: CellLevel) -> Option<CellLevel> {
    let storey = (*cell.level()).checked_add(1)?;
    if storey >= MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(cell.cell(), Level::new(storey)))
}

/// Whether the shooter is braced against terrain.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBraced(bool);

impl TerrainBraced {
    /// Build from a boolean.
    #[must_use]
    pub const fn new(braced: bool) -> Self {
        Self(braced)
    }
}
