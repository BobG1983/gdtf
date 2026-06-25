//! The static-terrain marker [`TerrainKind`] for one `(cell, level)` slot.

use serde::Deserialize;

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
}

impl TerrainKind {
    /// Whether this terrain kind blocks **on its own** — `true` for [`Wall`] and
    /// [`Cover`], `false` for [`Open`].
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
    pub const fn blocks(self) -> bool {
        matches!(self, Self::Wall | Self::Cover)
    }
}
