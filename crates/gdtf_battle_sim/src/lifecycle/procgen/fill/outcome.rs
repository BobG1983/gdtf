//! Result of filling the board after player/enemy placement.

use super::super::{
    assembler::{PlacedPrefab, Placement},
    geometry::RegionRect,
};

/// Player/enemy placement plus fill prefabs and leftover dead space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledPlacement {
    pub(super) placement:  Placement,
    pub(super) fill:       Vec<PlacedPrefab>,
    pub(super) dead_space: Vec<RegionRect>,
}

impl FilledPlacement {
    /// Build from parts.
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn new(
        placement: Placement,
        fill: Vec<PlacedPrefab>,
        dead_space: Vec<RegionRect>,
    ) -> Self {
        Self {
            placement,
            fill,
            dead_space,
        }
    }

    /// Player and enemy placements.
    #[must_use]
    pub const fn placement(&self) -> &Placement {
        &self.placement
    }

    /// Prefabs used to fill free space.
    #[must_use]
    pub fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

    /// Unfilled free rects left after fill.
    #[must_use]
    pub fn dead_space(&self) -> &[RegionRect] {
        &self.dead_space
    }
}
