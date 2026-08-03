//! Placed footprint records for debug / staging views.

use super::super::geometry::RegionRect;

/// Why a footprint was placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementRole {
    /// Player spawn.
    Player,
    /// Enemy spawn.
    Enemy,
    /// Fill content.
    Fill,
}

/// One placed region with its role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedFootprint {
    role: PlacementRole,
    region: RegionRect,
}

impl PlacedFootprint {
    /// Build a footprint record.
    #[must_use]
    pub const fn new(role: PlacementRole, region: RegionRect) -> Self {
        Self { role, region }
    }

    /// Placement role.
    #[must_use]
    pub const fn role(&self) -> PlacementRole {
        self.role
    }

    /// Board region.
    #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}
