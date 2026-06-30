//! Coarse 3D occupancy grid: the model's `(cell, level)` collision/query surface —
//! the static terrain plus the live occupant of every slot of the 60×60×8 coarse
//! grid (`docs/combat/battle-space.md`: "the 60×60×8 coarse grid"). It is the
//! **coarse occupancy** the authoritative model owns (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! This is the E1.6 occupancy-grid slice. The grid is what the collision/query
//! surface movement (GTW-12) and the LOS/cover queries read: each `(cell, level)`
//! slot carries a **static-terrain marker** ([`TerrainKind`] — wall / cover
//! presence, blocking vs non-blocking) and an **occupant marker**
//! ([`OccupancySlot::occupant`], an `Option<`[`Entity`](bevy::prelude::Entity)`>`).
//! The occupant is ALWAYS a Bevy [`Entity`](bevy::prelude::Entity) handle, **never a
//! numeric id** — the GTW-10 / GTW-12 architectural constraint that ids never cross
//! as raw integers into the grid.
//!
//! The occupancy is built **from the situation's static terrain plus live ganger
//! state**: always correct by construction, with a destroyed-cover set that keeps
//! smashed walls/props from resurrecting. This module supplies both halves:
//!
//! 1. [`OccupancyGrid::build_from_occupancy_input`] pours an [`OccupancyInput`]'s
//!    terrain and occupant placements into a fresh grid — the grid's constructor.
//!    The full situation→entities setup orchestration is GTW-158 (E1.8), which owns
//!    the canonical authored [`crate::situation::Situation`]; the [`OccupancyInput`]
//!    shape here is the **grid-relevant slice only** (terrain placements + occupant
//!    placements carrying [`Entity`](bevy::prelude::Entity) handles) that the setup
//!    derives from the spawned ganger entities and the authored walls/scatter.
//! 2. [`OccupancyGrid::destroyed_cover`] is an **append-only** exclusion set: a cell
//!    marked via [`OccupancyGrid::mark_cover_destroyed`] can never resurrect, and it
//!    is excluded from the blocking query [`OccupancyGrid::is_blocked`] (a destroyed
//!    cover cell does NOT block). This is the occupancy grid's **own** exclusion
//!    set, **distinct** from the GTW-154 [`crate::cover::CoverLedger`]'s HP-depletion
//!    [`crate::cover::Destroyed`] flag — syncing the two is GTW-157 (E1.7), not here.
//!
//! Reading on TOP of the grid (E7 · GTW-12b, ADR-0005) is the pure
//! [`pathable_neighbors`] enumeration: the same-storey 8-connected planar
//! neighbours of a `(cell, level)` that are in-bounds and walkable, each priced at
//! its step cost (orthogonal terrain `move_cost`, diagonal octile
//! `round(move_cost × √2)`), in deterministic `(z, y, x)` order. It is the planar
//! half of the route core's edge model; the cross-storey half is GTW-351's
//! [`crate::vertical::traversable_links`], and route assembly over both is GTW-352.
//!
//! The grid dimensions are STRUCTURAL constants: [`GRID_WIDTH`] / [`GRID_HEIGHT`]
//! (60×60, introduced here from `docs/combat/battle-space.md`) ×
//! [`MAX_LEVELS`](crate::metric::MAX_LEVELS) (8, from E1.1). Out-of-range
//! coordinates are handled **gracefully** — a query/mark on a cell outside the grid
//! is a no-op / "not blocked", never a panic.

mod grid;
mod input;
mod kind;
mod neighbours;
mod path_blocking;

#[cfg(test)]
mod test;

#[cfg(test)]
mod path_blocking_test;

pub use grid::{
    DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancySlot, StairEyeOffset,
};
pub use input::{OccupancyInput, OccupantPlacement, TerrainPlacement};
pub use kind::TerrainKind;
pub use neighbours::pathable_neighbors;
pub use path_blocking::{PathBlocking, project_path_blocking};
