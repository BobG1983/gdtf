//! Per-squad visible and explored cell sets.

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Resource},
};

use crate::{metric::CellLevel, occupancy::OccupancyGrid};

/// Whether a cell is currently visible to the squad.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellVisible(bool);

impl CellVisible {
    #[must_use]
    pub const fn new(visible: bool) -> Self {
        Self(visible)
    }
}

/// Whether a cell has ever been explored by the squad.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellExplored(bool);

impl CellExplored {
    #[must_use]
    pub const fn new(explored: bool) -> Self {
        Self(explored)
    }
}

/// Whether a ganger at a cell is visible to the squad.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GangerVisible(bool);

impl GangerVisible {
    #[must_use]
    pub const fn new(visible: bool) -> Self {
        Self(visible)
    }
}

/// Relation of an occupant to the planning squad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactionRelation {
    /// Same squad as the planner.
    OwnSquad,
    /// Any other faction.
    Other,
}

/// Visible and explored cells for one squad.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct SquadVisibility {
    visible: HashSet<CellLevel>,
    explored: HashSet<CellLevel>,
}

impl SquadVisibility {
    /// Build from explicit visible and explored sets.
    #[must_use]
    pub const fn new(visible: HashSet<CellLevel>, explored: HashSet<CellLevel>) -> Self {
        Self { visible, explored }
    }

    /// Everything visible and explored (AI move fog).
    #[must_use]
    pub fn omniscient(grid: &OccupancyGrid) -> Self {
        let all: HashSet<CellLevel> = grid.all_cells().collect();
        Self::new(all.clone(), all)
    }

    /// Whether the cell is currently in FOV.
    #[must_use]
    pub fn is_cell_visible(&self, cell: &CellLevel) -> CellVisible {
        CellVisible::new(self.visible.contains(cell))
    }

    /// Whether the cell has been explored at some point.
    #[must_use]
    pub fn is_cell_explored(&self, cell: &CellLevel) -> CellExplored {
        CellExplored::new(self.explored.contains(cell))
    }

    /// Iterator over currently visible cells.
    pub fn visible_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.visible.iter()
    }

    /// Iterator over all explored cells.
    pub fn explored_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.explored.iter()
    }
}

/// Own-squad members are always visible; others only when their cell is in FOV.
#[must_use]
pub fn is_ganger_visible(
    squad: &SquadVisibility,
    target: &CellLevel,
    relation: FactionRelation,
) -> GangerVisible {
    match relation {
        FactionRelation::OwnSquad => GangerVisible::new(true),
        FactionRelation::Other => GangerVisible::new(*squad.is_cell_visible(target)),
    }
}
