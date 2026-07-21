//! The two **deployment zones** the generated map surfaces (GTW-744) — the player and
//! enemy prefab regions plus the anchor→facing mapping the deploy step reads.

use super::super::{anchor::Anchor, geometry::RegionRect};
use crate::ganger::Direction;

/// One **deployment zone** — a placed prefab's [`Anchor`] plus its board
/// [`RegionRect`], the ground-plane rectangle a side's roster deploys into (GTW-744).
///
/// A named struct (no-bare-types: a deployment zone is a domain value, not a bare
/// anchor+rect tuple). Surfaced from the assembler's placement so
/// [`deploy_rosters`](super::deploy_rosters) can enumerate the zone's cells and derive a
/// side-appropriate facing from its anchor. All fields are `Copy`, so the zone is `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZone {
    /// The anchor the zone's prefab was placed flush against — the deploy step maps it to a
    /// facing pointing toward the board centre (the opposing zone).
    anchor: Anchor,
    /// The zone's ground-plane rectangle on the board (the prefab's placed footprint).
    region: RegionRect,
}

impl DeploymentZone {
    /// Build a deployment zone from its anchor and placed region.
    #[must_use]
    pub const fn new(anchor: Anchor, region: RegionRect) -> Self {
        Self { anchor, region }
    }

    /// The anchor the zone sits at.
    #[must_use]
    pub const fn anchor(self) -> Anchor {
        self.anchor
    }

    /// The zone's placed region on the board.
    #[must_use]
    pub const fn region(self) -> RegionRect {
        self.region
    }

    /// The [`Direction`] a member deployed in this zone faces — toward the board centre
    /// (the opposing zone), derived from the zone's [`anchor`](DeploymentZone::anchor).
    #[must_use]
    pub const fn facing(self) -> Direction {
        facing_for_anchor(self.anchor)
    }
}

/// The two opposing **deployment zones** the assembler produced — the player-spawn and
/// enemy-spawn prefab regions (GTW-744).
///
/// A named struct (no-bare-types: the pair of zones is a domain value). Rides back on
/// [`EmittedLevel::zones`](crate::procgen::EmittedLevel) so the app-side procgen driver can
/// hand them to [`deploy_rosters`](super::deploy_rosters). `Copy` (both zones are `Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentZones {
    /// The player-spawn zone (at the RNG-chosen anchor).
    player: DeploymentZone,
    /// The enemy-spawn zone (at the strict-opposite anchor).
    enemy:  DeploymentZone,
}

impl DeploymentZones {
    /// Build the deployment-zone pair from the player and enemy zones.
    #[must_use]
    pub const fn new(player: DeploymentZone, enemy: DeploymentZone) -> Self {
        Self { player, enemy }
    }

    /// The player-spawn zone.
    #[must_use]
    pub const fn player(self) -> DeploymentZone {
        self.player
    }

    /// The enemy-spawn zone.
    #[must_use]
    pub const fn enemy(self) -> DeploymentZone {
        self.enemy
    }
}

/// The [`Direction`] a member at `anchor` faces — toward the board centre, i.e. toward the
/// OPPOSITE (enemy) zone (GTW-744).
///
/// Derived purely from the anchor's coordinate meaning (the square grid's screen-free
/// convention: North is `-Y`, East is `+X` — see [`Direction`]). A member flush against a
/// corner faces the two-axis diagonal back toward centre; against an edge-middle, the one
/// cardinal. Kept here in `deploy` (not on [`Anchor`]) so the anchor module stays
/// geometry-only — facing is a deployment concern.
#[must_use]
pub const fn facing_for_anchor(anchor: Anchor) -> Direction {
    match anchor {
        // Corners face the diagonal back toward the board centre.
        Anchor::TopRight => Direction::NorthWest,
        Anchor::TopLeft => Direction::NorthEast,
        Anchor::BottomRight => Direction::SouthWest,
        Anchor::BottomLeft => Direction::SouthEast,
        // Edge-middles face the one cardinal toward the opposite edge.
        Anchor::RightMiddle => Direction::West,
        Anchor::LeftMiddle => Direction::East,
        Anchor::TopMiddle => Direction::North,
        Anchor::BottomMiddle => Direction::South,
    }
}
