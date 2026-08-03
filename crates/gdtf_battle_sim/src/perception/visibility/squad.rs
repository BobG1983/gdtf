use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Resource},
};

use crate::{metric::CellLevel, occupancy::OccupancyGrid};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellVisible(bool);

impl CellVisible {
        #[must_use]
    pub const fn new(visible: bool) -> Self {
        Self(visible)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellExplored(bool);

impl CellExplored {
        #[must_use]
    pub const fn new(explored: bool) -> Self {
        Self(explored)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GangerVisible(bool);

impl GangerVisible {
        #[must_use]
    pub const fn new(visible: bool) -> Self {
        Self(visible)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactionRelation {
            OwnSquad,
            Other,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct SquadVisibility {
            visible:  HashSet<CellLevel>,
                explored: HashSet<CellLevel>,
}

impl SquadVisibility {
                                #[must_use]
    pub const fn new(visible: HashSet<CellLevel>, explored: HashSet<CellLevel>) -> Self {
        Self { visible, explored }
    }

                                                                #[must_use]
    pub fn omniscient(grid: &OccupancyGrid) -> Self {
        let all: HashSet<CellLevel> = grid.all_cells().collect();
        Self::new(all.clone(), all)
    }

                #[must_use]
    pub fn is_cell_visible(&self, cell: &CellLevel) -> CellVisible {
        CellVisible::new(self.visible.contains(cell))
    }

                #[must_use]
    pub fn is_cell_explored(&self, cell: &CellLevel) -> CellExplored {
        CellExplored::new(self.explored.contains(cell))
    }

                    pub fn visible_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.visible.iter()
    }

                        pub fn explored_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.explored.iter()
    }
}

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
