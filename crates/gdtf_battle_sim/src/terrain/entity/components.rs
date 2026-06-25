//! The per-terrain-entity **identity components** — [`TerrainCell`] (the cell-coordinate
//! component keying every terrain entity to its `(cell, level)` position) and
//! [`TerrainPieceKind`] (the marker enum naming whether the entity is a `Wall`, `Cover`
//! scatter prop, or `Slab`). GTW-395.

use bevy::prelude::{Component, Deref};

use crate::metric::CellLevel;

/// The `(cell, level)` position component on every terrain entity — the ECS side
/// of the cell-to-entity mapping, complementing the [`TerrainIndex`](super::index::TerrainIndex)
/// resource's entity-to-cell reverse lookup.
///
/// A `CellLevel` newtype wrapping component (no-bare-types: a terrain cell position is
/// a domain value, not a bare `IVec3`). Private inner + derived [`Deref`] (house style).
/// No `Default` — terrain cells are always constructed from an authored `(cell, level)`
/// in the setup loop (via `commands.spawn((TerrainCell::new(..),  ..))`, not `bsn!`);
/// there is no meaningful origin-sentinel default for a terrain piece cell.
///
/// One [`TerrainCell`] per terrain entity; the entity's [`TerrainPieceKind`] (on the
/// same entity) says whether this cell belongs to a wall, scatter prop, or slab.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainCell(CellLevel);

impl TerrainCell {
    /// Build a terrain cell from its `(cell, level)` position.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// Marks a slab ECS entity as a **stair-brace slab** — an intact slab directly above
/// the LOWER endpoint of an authored stair, under which a kneeling occupant braces
/// their weapon (GTW-392; `docs/combat/resolution.md` §1a brace gate, terrain-brace
/// clause).
///
/// A pure marker: it carries no data. The slab it braces is the cell directly below
/// it (`self_cell` at storey `n` ⇒ the brace stair cell is at storey `n − 1`), which is
/// the LOWER stair endpoint. The live [`crate::surface::SlabState::Present`] gate is
/// read from [`crate::surface::SurfaceGrid`] at fire time, not from this marker's
/// presence; a destroyed slab keeps its entity + marker but loses the brace.
///
/// Only the LOWER of a stair's two endpoints earns this: an occupant on the upper
/// arrival cell braces against their own storey's ceiling, which is ordinary cover,
/// not the stair-brace slab. See [`crate::terrain::slab::BraceStairCells`] for the
/// lower-endpoint cell set and `stability::terrain_brace::terrain_braces` for the live gate.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainBrace;

/// The kind of terrain piece this entity represents — `Wall`, `Cover` (scatter prop),
/// or `Slab` (floor / roof).
///
/// A named domain enum (no-bare-types: a terrain kind is not a bare discriminant
/// integer). The queryable kind tag on every terrain entity, recoverable from the
/// source list the setup loop iterated: walls come from `situation.walls`, scatter
/// props from `situation.scatter`, and slabs from `situation.slabs`. The `Floor`
/// variant is reserved for GTW-396 (walkable-floor entities with `MoveCost`) and is
/// not spawned by this ticket.
///
/// A Bevy [`Component`]: one `TerrainPieceKind` per terrain entity, queryable alongside
/// [`TerrainCell`] for the kind+cell lookup path (C4).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainPieceKind {
    /// A solid wall — blocks movement and provides HIGH cover (authored in
    /// `situation.walls`, seeded into the [`CoverLedger`](crate::cover::CoverLedger)).
    Wall,
    /// A scatter prop — destructible cover / obstacle (authored in
    /// `situation.scatter`, seeded into the [`CoverLedger`](crate::cover::CoverLedger)).
    Cover,
    /// A floor or roof slab — an impassable z-boundary until destroyed (authored in
    /// `situation.slabs`, existence tracked by the
    /// [`SurfaceGrid`](crate::surface::SurfaceGrid) and HP by the
    /// [`SlabLedger`](crate::slab::SlabLedger)).
    Slab,
}
