use super::super::{
    assembler::{PlacedPrefab, Placement},
    geometry::RegionRect,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledPlacement {
        pub(super) placement:  Placement,
        pub(super) fill:       Vec<PlacedPrefab>,
            pub(super) dead_space: Vec<RegionRect>,
}

impl FilledPlacement {
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

        #[must_use]
    pub const fn placement(&self) -> &Placement {
        &self.placement
    }

        #[must_use]
    pub fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

            #[must_use]
    pub fn dead_space(&self) -> &[RegionRect] {
        &self.dead_space
    }
}
