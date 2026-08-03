use super::super::geometry::RegionRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementRole {
        Player,
        Enemy,
        Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedFootprint {
        role:   PlacementRole,
        region: RegionRect,
}

impl PlacedFootprint {
        #[must_use]
    pub const fn new(role: PlacementRole, region: RegionRect) -> Self {
        Self { role, region }
    }

        #[must_use]
    pub const fn role(&self) -> PlacementRole {
        self.role
    }

        #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}
