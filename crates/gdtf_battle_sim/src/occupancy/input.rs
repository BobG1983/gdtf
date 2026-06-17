//! The grid-relevant construction input — the terrain and occupant placements
//! [`OccupancyGrid::build_from_occupancy_input`](crate::occupancy::OccupancyGrid::build_from_occupancy_input)
//! pours into a fresh grid.

use bevy::prelude::Entity;

use crate::{metric::CellLevel, occupancy::TerrainKind};

/// A terrain placement in an [`OccupancyInput`] — a `(cell, level)` slot and the
/// [`TerrainKind`] authored there.
///
/// The grid-relevant slice of a situation's static geometry (walls / cover poured
/// into the grid). A named struct rather than a bare `(CellLevel, TerrainKind)`
/// tuple so the grid's input shape is self-describing. The full situation
/// authoring/orchestration is GTW-158.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPlacement {
    /// The `(cell, level)` this terrain occupies.
    pub at:      CellLevel,
    /// The kind of terrain authored at [`at`](TerrainPlacement::at).
    pub terrain: TerrainKind,
}

impl TerrainPlacement {
    /// Build a terrain placement from its `(cell, level)` and [`TerrainKind`].
    #[must_use]
    pub const fn new(at: CellLevel, terrain: TerrainKind) -> Self {
        Self { at, terrain }
    }
}

/// An occupant placement in an [`OccupancyInput`] — a `(cell, level)` slot and the
/// Bevy [`Entity`] standing there.
///
/// The grid-relevant slice of a situation's live occupants (gangers poured into the
/// grid). Carries an [`Entity`] handle, **never a numeric id** (GTW-10 / GTW-12). A
/// named struct rather than a bare `(CellLevel, Entity)` tuple so the input shape is
/// self-describing. The full situation authoring/orchestration is GTW-158.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccupantPlacement {
    /// The `(cell, level)` the occupant stands in.
    pub at:       CellLevel,
    /// The entity occupying [`at`](OccupantPlacement::at) — a Bevy [`Entity`], never
    /// a numeric id.
    pub occupant: Entity,
}

impl OccupantPlacement {
    /// Build an occupant placement from its `(cell, level)` and the occupying
    /// [`Entity`].
    #[must_use]
    pub const fn new(at: CellLevel, occupant: Entity) -> Self {
        Self { at, occupant }
    }
}

/// The grid-relevant **construction input** for the coarse occupancy grid — the
/// terrain and occupant placements
/// [`OccupancyGrid::build_from_occupancy_input`](crate::occupancy::OccupancyGrid::build_from_occupancy_input)
/// pours into a fresh grid.
///
/// This is the INPUT shape E1.6 defines (renamed from the GTW-156 placeholder
/// `Situation` by GTW-158, which makes the authored [`crate::situation::Situation`]
/// the one canonical situation type): the static-terrain placements
/// ([`TerrainPlacement`]) and the live-occupant placements ([`OccupantPlacement`],
/// each carrying an [`Entity`] handle). It is deliberately the **grid-relevant
/// slice only** — the GTW-158 setup (E1.8) derives an `OccupancyInput` from the
/// canonical situation (terrain from the authored walls / scatter, occupants from
/// the SPAWNED ganger entities) and pours it through here.
#[derive(Debug, Clone, Default)]
pub struct OccupancyInput {
    /// The static-terrain placements (walls / cover) to pour into the grid.
    pub terrain:   Vec<TerrainPlacement>,
    /// The live-occupant placements (entities) to pour into the grid.
    pub occupants: Vec<OccupantPlacement>,
}

impl OccupancyInput {
    /// Build an empty occupancy input (no terrain, no occupants).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
