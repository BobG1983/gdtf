use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Entity},
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    occupancy::TerrainKind,
};

pub const GRID_WIDTH: usize = 60;

pub const GRID_HEIGHT: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OccupancySlot {
        pub terrain:  TerrainKind,
            pub occupant: Option<Entity>,
}

pub(super) const SLOT_COUNT: usize = GRID_WIDTH * GRID_HEIGHT * (MAX_LEVELS as usize);

#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct DestroyedCover(HashSet<CellLevel>);

impl DestroyedCover {
        #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

                    pub fn mark(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blocked(bool);

impl Blocked {
        #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathBlocked(bool);

impl PathBlocked {
        #[must_use]
    pub const fn new(blocked: bool) -> Self {
        Self(blocked)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccludesVision(bool);

impl OccludesVision {
        #[must_use]
    pub const fn new(occludes: bool) -> Self {
        Self(occludes)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed(bool);

impl CoverDestroyed {
        #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}
