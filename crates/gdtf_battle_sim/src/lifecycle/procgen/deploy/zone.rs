//! Player and enemy deployment zones.

use super::super::{anchor::Anchor, geometry::RegionRect};
use crate::ganger::Direction;

/// One side's deployment region and anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZone {
    anchor: Anchor,
    region: RegionRect,
}

impl DeploymentZone {
    /// From anchor and region.
    #[must_use]
    pub const fn new(anchor: Anchor, region: RegionRect) -> Self {
        Self { anchor, region }
    }

    /// Edge/corner this zone sits on.
    #[must_use]
    pub const fn anchor(self) -> Anchor {
        self.anchor
    }

    /// Board region of the zone.
    #[must_use]
    pub const fn region(self) -> RegionRect {
        self.region
    }

    /// Default facing for units in this zone.
    #[must_use]
    pub const fn facing(self) -> Direction {
        facing_for_anchor(self.anchor)
    }
}

/// Player and enemy zones together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZones {
    player: DeploymentZone,
    enemy:  DeploymentZone,
}

impl DeploymentZones {
    /// Pair of zones.
    #[must_use]
    pub const fn new(player: DeploymentZone, enemy: DeploymentZone) -> Self {
        Self { player, enemy }
    }

    /// Player zone.
    #[must_use]
    pub const fn player(self) -> DeploymentZone {
        self.player
    }

    /// Enemy zone.
    #[must_use]
    pub const fn enemy(self) -> DeploymentZone {
        self.enemy
    }
}

/// Facing direction implied by an anchor.
#[must_use]
pub const fn facing_for_anchor(anchor: Anchor) -> Direction {
    match anchor {
        Anchor::TopRight => Direction::NorthWest,
        Anchor::TopLeft => Direction::NorthEast,
        Anchor::BottomRight => Direction::SouthWest,
        Anchor::BottomLeft => Direction::SouthEast,
        Anchor::RightMiddle => Direction::West,
        Anchor::LeftMiddle => Direction::East,
        Anchor::TopMiddle => Direction::North,
        Anchor::BottomMiddle => Direction::South,
    }
}
