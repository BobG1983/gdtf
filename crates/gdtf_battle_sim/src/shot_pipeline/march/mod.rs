//! The §2 projectile travel — the **true 3-axis voxel DDA** ([`march_vector`]) that
//! flies one 3D shot ray through the 60×60×8 cubic-voxel grid and reports the first
//! thing the round fails to clear (`docs/combat/resolution.md` §2 + §3;
//! `docs/combat/battle-space.md` §"The shot is one 3D Vec3, ray-marched by voxel
//! DDA").
//!
//! This is the E2.7 slice. The shot is **ONE 3D `Vec3`** — a continuous muzzle
//! [`SimPos`](crate::metric::SimPos) plus a unit-`Vec3` direction — marched as an
//! **Amanatides–Woo** voxel DDA in **sim units** (one cell on x = one cell on y =
//! one level on z = 1.0): [`march_vector`] walks the grid cell-by-cell and
//! level-by-level, never a pixel-stepped ray, and at each thing it crosses it applies
//! the E2.6 clearance rule (`docs/combat/resolution.md` §2: **strictly-higher SAILS
//! OVER / equal-or-lower IMPACTS**). **Zero pixels.**
//!
//! What the march tests, in order, as it walks the ray.
//!
//! **Occupied `(cell, level)`** — a ganger occupant
//! ([`OccupancyGrid::occupant`](crate::occupancy::OccupancyGrid::occupant) plus its
//! silhouette band from
//! [`OccupancyGrid::occupant_band`](crate::occupancy::OccupancyGrid::occupant_band))
//! OR a piece of standing cover ([`crate::cover::CoverEntry`] read from
//! [`CoverLedger`](crate::cover::CoverLedger), excluded when destroyed via
//! [`OccupancyGrid::is_blocked`](crate::occupancy::OccupancyGrid::is_blocked) /
//! [`OccupancyGrid::is_cover_destroyed`](crate::occupancy::OccupancyGrid::is_cover_destroyed)).
//! The round's band at the crossing is
//! [`round_band_for_cell`](crate::clearance::round_band_for_cell) (its continuous z
//! within the crossed level), and
//! [`round_clears_occupant`](crate::clearance::round_clears_occupant) decides: a
//! strictly higher round sails over and the march continues, an equal-or-lower round
//! impacts and the march stops. **Any** actor in the path impacts — **including the
//! shooter's own gang** (true friendly fire): the rule is band-vs-band, with no
//! exemption list.
//!
//! **Z-boundary crossings** — when the DDA steps across a storey boundary, the
//! floor/roof [`SurfaceGrid`](crate::surface::SurfaceGrid) slab is tested (one slab,
//! both faces): a [`SlabState::Present`](crate::surface::SlabState::Present) slab
//! stops the round; [`SlabState::Destroyed`](crate::surface::SlabState::Destroyed) /
//! [`SlabState::Absent`](crate::surface::SlabState::Absent) passes.
//!
//! **Grid exits** — leaving the **top** is a clean sky [`MarchKind::Miss`]; leaving
//! the **bottom** strikes the [`MarchKind::Ground`] (damaged, never destroyed);
//! leaving **laterally** is a [`MarchKind::Miss`].
//!
//! There is **NO target stop** — the round flies past the aim cell into whatever is
//! behind it; a "miss" is just a shot whose deviation carried it past everything.
//! The **only** hard exception is that the **shooter's own cell never blocks its own
//! shot** (`docs/combat/resolution.md` §2). Degenerate marches (a direction that
//! leaves the grid immediately, a zero direction) are graceful — a [`MarchKind::Miss`],
//! never a panic.

mod arc;
mod dda;
mod geom;
mod result;
mod vector;

#[cfg(test)]
mod test;

// GTW-546 (child GTW-41d): the lobbed-grenade arc march — the `TrajectoryStyle::Arc`
// counterpart to `march_vector`. A deterministic parabola blocked only by intact roofs
// (holes / windows pass); its landing feeds the GTW-541 blast resolver.
pub use arc::march_arc;
pub use result::{MarchKind, MarchResult};
pub use vector::march_vector;
