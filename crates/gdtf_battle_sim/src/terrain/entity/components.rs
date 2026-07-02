//! The per-terrain-entity **identity components** — [`TerrainCell`] (the cell-coordinate
//! component keying every terrain entity to its `(cell, level)` position) and
//! [`TerrainPieceKind`] (the marker enum naming whether the entity is a `Wall`, `Cover`
//! scatter prop, or `Slab`). GTW-395.

use bevy::prelude::{Component, Deref};

use crate::{cover::HeightBand, metric::CellLevel};

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

/// Marks a terrain ECS entity as **path-blocking** — the sim's pathfinding treats its
/// `(cell, level)` as impassable (GTW-501, child 482a of the tag-driven-terrain epic).
///
/// A pure marker (a unit struct, no data — no-bare-types rule 4 carve-out: a marker
/// component carries no domain value). It is the SOURCE OF TRUTH for path-blocking:
/// attached at terrain-entity spawn, derived from the piece's
/// [`TerrainDef`](crate::terrain::def::TerrainDef) by
/// [`derives_path_blocking`](crate::terrain::def::derives_path_blocking) — present iff the
/// def carries an explicit [`BlocksPathfinding`](crate::terrain::def::TerrainTag::BlocksPathfinding)
/// tag OR its [`sim_kind`](crate::terrain::def::TerrainSimKind) defaults to path-blocking
/// (`Wall` / `Cover` block by default; `Slab` does not). An explicit tag therefore ADDS
/// path-blocking to an otherwise-open `Slab`, and existing walls/cover keep blocking with
/// no content migration (the GTW-501 zero-regression rule, D2).
///
/// The [`OccupancyGrid`](crate::occupancy::OccupancyGrid)'s tag-derived path-blocking
/// surface is the PROJECTED snapshot of these markers:
/// [`project_path_blocking`](crate::occupancy::project_path_blocking) folds the markers
/// into the grid at setup and keeps them in sync via
/// `Added<BlocksPathfinding>` / `RemovedComponents<BlocksPathfinding>` change detection.
/// The pathfinder reads that surface, NEVER branching on
/// [`TerrainPieceKind`](TerrainPieceKind) directly — satisfying the epic criterion
/// "queries read tags, not kind". This is PATH-blocking ONLY: vision still reads the
/// kind-based [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked)
/// (the split is GTW-501 D1; vision's own occluder is GTW-502).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlocksPathfinding;

/// Marks a terrain ECS entity as **vision-occluding**, carrying the [`HeightBand`] at which
/// it occludes line-of-sight / field-of-view (GTW-502, child 482b of the tag-driven-terrain
/// epic GTW-482).
///
/// NOT a bare marker — per the user-pinned GTW-482 refinement, LoS/FoV blocking is
/// **height-aware**: a component carrying a HEIGHT enum (the band it occludes), consistent
/// with the existing height-aware band-LoS model. So this is a single-field tuple
/// [`Component`] wrapping a [`HeightBand`] (no-bare-types: the band IS the domain value the
/// component exists to carry, exposed read-only through the derived [`Deref`] — the inner is
/// private, constructed via [`new`](BlocksVision::new)). A LOW occluder blocks a LOW eye-line
/// but a round/eye-line one band HIGHER sails over it, exactly as a
/// [`CoverEntry`](crate::cover::CoverEntry)'s band gates the march.
///
/// It is the SOURCE OF TRUTH for vision occlusion: attached at terrain-entity spawn, derived
/// from the piece's [`TerrainDef`](crate::terrain::def::TerrainDef) by
/// [`derives_vision_occlusion`](crate::terrain::def::derives_vision_occlusion) — present iff
/// the def carries an explicit [`BlocksVision`](crate::terrain::def::TerrainTag::BlocksVision)
/// tag OR its [`sim_kind`](crate::terrain::def::TerrainSimKind) occludes by default
/// (`Wall`/`Cover` occlude at their band; `Slab` does not). An explicit tag therefore ADDS
/// vision occlusion to an otherwise-transparent `Slab`, and existing walls/cover keep
/// occluding with no content migration (the GTW-502 zero-regression rule — a `Wall`/`Cover`
/// already occludes sight via its [`CoverLedger`](crate::cover::CoverLedger) entry, and this
/// derives the SAME band, an idempotent re-block, never a double-count).
///
/// The [`OccupancyGrid`](crate::occupancy::OccupancyGrid)'s tag-derived vision-blocking
/// surface ([`VisionBlocking`](crate::occupancy::VisionBlocking)) is the PROJECTED snapshot
/// of these components:
/// [`project_vision_blocking`](crate::occupancy::project_vision_blocking) folds the
/// components into the grid at setup and keeps them in sync via
/// `Added<BlocksVision>` / `Changed<BlocksVision>` / `RemovedComponents<BlocksVision>` change
/// detection. [`impact_at`](crate::march) reads that surface alongside the occupant + cover
/// checks (height-aware). This is VISION occlusion ONLY: the GTW-501 path-blocking
/// ([`BlocksPathfinding`]) surface is INDEPENDENT — a path-only blocker does not occlude
/// vision and a vision-only occluder does not block a path.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlocksVision(HeightBand);

impl BlocksVision {
    /// Build a vision-occluder marker that occludes at `band` — the band derived from the
    /// piece's def by
    /// [`derives_vision_occlusion`](crate::terrain::def::derives_vision_occlusion).
    #[must_use]
    pub const fn new(band: HeightBand) -> Self {
        Self(band)
    }
}

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
    /// A weapon emplacement (GTW-543) — a cover-like smashable structure a ganger can
    /// ENTER to operate a mounted gun (authored in `situation.walls`, seeded into the
    /// [`CoverLedger`](crate::cover::CoverLedger) like a wall/cover). It additionally
    /// carries the enter/exit
    /// [`EmplacementState`](crate::terrain::emplacement::EmplacementState) + a
    /// [`MountedWeaponKey`](crate::terrain::emplacement::MountedWeaponKey).
    Emplacement,
}
