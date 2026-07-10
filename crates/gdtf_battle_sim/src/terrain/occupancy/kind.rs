//! The static-terrain marker [`TerrainKind`] for one `(cell, level)` slot.

use serde::Deserialize;

use crate::{occupancy::Blocked, terrain::entity::TerrainPieceKind};

/// The static-terrain marker for one `(cell, level)` slot — what kind of fixed
/// terrain occupies it, and therefore whether it **blocks** (collision / LOS /
/// cover).
///
/// A named domain enum (no-bare-types: terrain presence is a domain value, not a
/// bare `Option<bool>`), carrying the two categories `docs/combat/combat.md` names
/// for fixed geometry — walls and cover — alongside open space:
///
/// - [`Open`](TerrainKind::Open): empty space — no fixed terrain, **non-blocking**.
/// - [`Wall`](TerrainKind::Wall): a wall — solid fixed geometry, **blocking**
///   (`docs/combat/combat.md`: walls are part of the coarse geometry a shot/LOS
///   flies through).
/// - [`Cover`](TerrainKind::Cover): a piece of cover (wall-height or a prop) present
///   in this slot — **blocking** while it stands, but excluded once the cell is in
///   the [`destroyed_cover`](crate::occupancy::OccupancyGrid::destroyed_cover) set
///   (`docs/combat/combat.md`: cover "can be shot and destroyed").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum TerrainKind {
    /// Empty space — no fixed terrain here; **non-blocking**.
    #[default]
    Open,
    /// A wall — solid fixed geometry; **blocking**.
    Wall,
    /// A piece of cover present in this slot — **blocking** until the cell is marked
    /// destroyed (then excluded from the blocking query).
    Cover,
    /// A weapon emplacement present in this slot (GTW-543) — a cover-like smashable
    /// structure a ganger can enter; **blocking** while it stands (like
    /// [`Cover`](TerrainKind::Cover)), excluded once the cell is marked destroyed.
    Emplacement,
}

impl TerrainKind {
    /// Whether this terrain kind blocks **on its own** — `true` for [`Wall`],
    /// [`Cover`], and [`Emplacement`](TerrainKind::Emplacement); `false` for [`Open`].
    ///
    /// This is the *static* blocking-ness of the terrain marker alone; it does NOT
    /// account for the destroyed-cover exclusion (a destroyed [`Cover`] cell still
    /// reads `true` here but is excluded by
    /// [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked)).
    /// Callers wanting the live, destruction-aware answer use
    /// [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked).
    ///
    /// [`Wall`]: TerrainKind::Wall
    /// [`Cover`]: TerrainKind::Cover
    /// [`Open`]: TerrainKind::Open
    #[must_use]
    pub const fn blocks(self) -> Blocked {
        Blocked::new(matches!(self, Self::Wall | Self::Cover | Self::Emplacement))
    }
}

impl From<TerrainPieceKind> for TerrainKind {
    /// Map a terrain ENTITY's [`TerrainPieceKind`] (the spec-variant-derived kind a
    /// resolved cover/slab piece carries) to the occupancy-grid [`TerrainKind`] marker
    /// for its `(cell, level)` slot.
    ///
    /// The two enums are distinct concerns — [`TerrainPieceKind`] tags the spawned
    /// terrain entity (`Wall` / `Cover` / `Slab`), while [`TerrainKind`] is the coarse
    /// occupancy slot marker the collision / LOS / cover queries read. This is the one
    /// authoritative bridge so a piece's occupancy is derived from the def's OWN kind
    /// (GTW-483) rather than from which authoring list it happened to sit in:
    ///
    /// - [`Wall`](TerrainPieceKind::Wall) → [`Wall`](TerrainKind::Wall).
    /// - [`Cover`](TerrainPieceKind::Cover) → [`Cover`](TerrainKind::Cover).
    /// - [`Emplacement`](TerrainPieceKind::Emplacement) →
    ///   [`Emplacement`](TerrainKind::Emplacement): a cover-like smashable structure that
    ///   blocks its slot until destroyed (GTW-543).
    /// - [`Slab`](TerrainPieceKind::Slab) → [`Open`](TerrainKind::Open): a slab is a
    ///   floor / roof z-boundary tracked by the [`SurfaceGrid`](crate::surface::SurfaceGrid),
    ///   not a blocking marker in the `(cell, level)` occupancy slot, so it leaves the
    ///   slot open.
    fn from(kind: TerrainPieceKind) -> Self {
        match kind {
            TerrainPieceKind::Wall => Self::Wall,
            TerrainPieceKind::Cover => Self::Cover,
            TerrainPieceKind::Emplacement => Self::Emplacement,
            TerrainPieceKind::Slab => Self::Open,
        }
    }
}
