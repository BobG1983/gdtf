//! The **schematic footprint** — the render-free per-placement data an app-side stepper
//! overview draws (GTW-732): the role a placed prefab plays and the board [`RegionRect`] it
//! occupies.
//!
//! This is MODEL data, not a render primitive: [`ProcgenCursor`](super::cursor::ProcgenCursor)
//! exposes the placements it has landed so far as a list of these, and an app-side egui
//! painter maps each [`region`](PlacedFootprint::region) onto a scaled grid overview (NOT the
//! real terrain renderer). Keeping it here — beside the cursor that produces it — keeps the
//! sim render-free while still giving the view a typed handle on "what has been placed".

use super::super::geometry::RegionRect;

/// Which role a placed prefab plays in the schematic overview (GTW-732).
///
/// A named domain enum (no-bare-types: the placement role is a domain value, not a bare
/// index/flag). The overview tints the player, enemy, and connective-fill footprints
/// distinctly so a developer can read the level assembling piece by piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementRole {
    /// The player-spawn deployment prefab (GTW-424 C1).
    Player,
    /// The enemy-spawn deployment prefab at the strict opposite (GTW-424 C2).
    Enemy,
    /// A connective same-theme fill prefab (GTW-427).
    Fill,
}

/// One placed prefab's schematic footprint — its [`PlacementRole`] and the board
/// [`RegionRect`] it occupies (GTW-732).
///
/// A named struct (no-bare-types: a schematic footprint is a domain value, not a bare
/// role+rect tuple). Produced by `ProcgenCursor::placed_footprints` as each placement lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedFootprint {
    /// The role of the placed prefab.
    role:   PlacementRole,
    /// Its placed region on the board.
    region: RegionRect,
}

impl PlacedFootprint {
    /// Build a schematic footprint from a role + placed region.
    #[must_use]
    pub const fn new(role: PlacementRole, region: RegionRect) -> Self {
        Self { role, region }
    }

    /// The role of the placed prefab.
    #[must_use]
    pub const fn role(&self) -> PlacementRole {
        self.role
    }

    /// Its placed region on the board.
    #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}
