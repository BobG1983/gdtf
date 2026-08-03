use super::super::{anchor::Anchor, geometry::RegionRect};
use crate::ganger::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZone {
            anchor: Anchor,
        region: RegionRect,
}

impl DeploymentZone {
        #[must_use]
    pub const fn new(anchor: Anchor, region: RegionRect) -> Self {
        Self { anchor, region }
    }

        #[must_use]
    pub const fn anchor(self) -> Anchor {
        self.anchor
    }

        #[must_use]
    pub const fn region(self) -> RegionRect {
        self.region
    }

            #[must_use]
    pub const fn facing(self) -> Direction {
        facing_for_anchor(self.anchor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZones {
        player: DeploymentZone,
        enemy:  DeploymentZone,
}

impl DeploymentZones {
        #[must_use]
    pub const fn new(player: DeploymentZone, enemy: DeploymentZone) -> Self {
        Self { player, enemy }
    }

        #[must_use]
    pub const fn player(self) -> DeploymentZone {
        self.player
    }

        #[must_use]
    pub const fn enemy(self) -> DeploymentZone {
        self.enemy
    }
}

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
