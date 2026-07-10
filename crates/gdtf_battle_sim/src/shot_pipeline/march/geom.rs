//! Stateless grid + ray geometry helpers shared by the DDA walk — the grid-bounds
//! tests, the `(cell, level)` key builders, and the point-along-the-ray evaluator
//! (sim units; one cell on x = one cell on y = one level on z = 1.0). Also the home of
//! the march's geometry newtypes: the [`VoxelIndex`] axis coordinate, the [`AxisStep`]
//! DDA step, the [`RayParam`] ray parameter, the [`MarchDir`] ray direction, and the
//! [`InGrid`] bounds answer.

use bevy::{math::Vec3, prelude::Deref};

use crate::{
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
};

/// A signed integer voxel index on ONE grid axis — a ground cell coordinate on
/// x / y, or a storey index on z.
///
/// Wraps `i32` (no-bare-types: a voxel-axis index is a domain coordinate, not a bare
/// integer) and is deliberately signed and unbounded: the DDA can step a voxel index
/// OUT of the grid (a lateral / top / bottom exit) before the bounds test rejects it,
/// so it must represent negative and past-the-edge values without wrapping. Private
/// inner + derived [`Deref`]; [`Ord`] so `min`/`max`/`<`/`>=` order two indices along
/// an axis.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VoxelIndex(i32);

impl VoxelIndex {
    /// Build a voxel-axis index from its signed integer coordinate.
    #[must_use]
    pub(super) const fn new(index: i32) -> Self {
        Self(index)
    }
}

/// The DDA per-axis **step direction** — `+1` / `-1` toward the next voxel on that
/// axis, or `0` when the ray does not move on it (Amanatides–Woo).
///
/// Wraps `i32` (no-bare-types: a step direction is a domain value, distinct from a
/// [`VoxelIndex`] coordinate it is added to) so the sign can never be confused with a
/// coordinate. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AxisStep(i32);

impl AxisStep {
    /// Build a per-axis step direction (`+1` / `-1` / `0`).
    #[must_use]
    pub(super) const fn new(step: i32) -> Self {
        Self(step)
    }
}

/// A value of the ray **parameter `t`** along a march ray (`muzzle + t × dir`), in
/// sim units — the `t` at which the ray crosses a voxel boundary (`t_max`), the `t`
/// span of one whole voxel (`t_delta`), or the `t` the ray entered a voxel at.
///
/// Wraps `f32` (no-bare-types: a ray parameter is a domain magnitude, distinct from a
/// sim-unit coordinate or a dimensionless fraction) so a `t` can never be confused
/// with a position. `f32::INFINITY` is a legal value (a non-moving axis). Private
/// inner + derived [`Deref`]; [`PartialOrd`] so the smallest-`t` axis is chosen by
/// comparison.
#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct RayParam(f32);

impl RayParam {
    /// Build a ray parameter `t` from its sim-unit magnitude.
    #[must_use]
    pub(super) const fn new(t: f32) -> Self {
        Self(t)
    }
}

/// One Cartesian **component** of a [`MarchDir`] unit direction — the per-axis
/// projection the DDA initialises each axis's traversal from.
///
/// Wraps `f32` (no-bare-types: a direction component is a dimensionless domain value,
/// distinct from a sim-unit coordinate) so a direction component can never be confused
/// with a position. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct AxisDir(f32);

impl AxisDir {
    /// Build a direction component from its dimensionless magnitude.
    #[must_use]
    pub(super) const fn new(component: f32) -> Self {
        Self(component)
    }
}

/// The unit **direction a voxel-DDA march ray travels** — a sampled shot trajectory
/// (`docs/combat/resolution.md` §2) or an eye→aim line-of-sight sight line
/// ([`crate::los`]), whichever the march is flying.
///
/// The generalised march input BOTH the shot pipeline and the LOS probe feed:
/// distinct from [`ShotDir`](crate::sample_cone::ShotDir) (one sampled shot) and
/// [`AimDir`](crate::central_axis::AimDir) (the central axis) — it is the ray the
/// grid walk marches, regardless of what produced it. Wraps `Vec3` (no-bare-types: a
/// march direction is a domain value, not a bare vector); only its direction matters
/// (a non-unit or zero vector is handled gracefully by [`march_vector`](super::march_vector)).
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct MarchDir(Vec3);

impl MarchDir {
    /// Build a march direction from a direction vector (used as-is; only its
    /// direction matters — a zero / non-unit vector is handled by the march).
    #[must_use]
    pub const fn new(dir: Vec3) -> Self {
        Self(dir)
    }
}

/// Whether a voxel-axis coordinate lies **inside** the coarse grid extent — the
/// answer the march's bounds tests ([`xy_in_grid`] / [`z_in_grid`]) and the `AoE`
/// template's edge-clamp return.
///
/// A named predicate newtype (no-bare-types: "in the grid" is a domain answer, not a
/// bare `bool`), distinct from the firing guard's full-3D
/// [`InBounds`](crate::magazine::InBounds): this answers a per-axis / ground-plane
/// membership the geometry layer asks while walking a ray or clamping a template.
/// Private inner, read through the derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InGrid(bool);

impl InGrid {
    /// Build an in-grid-bounds answer.
    #[must_use]
    pub const fn new(inside: bool) -> Self {
        Self(inside)
    }
}

/// Whether `(x, y)` is inside the ground-plane grid extent.
pub(super) fn xy_in_grid(x: VoxelIndex, y: VoxelIndex) -> InGrid {
    let Ok(ux) = usize::try_from(*x) else {
        return InGrid::new(false);
    };
    let Ok(uy) = usize::try_from(*y) else {
        return InGrid::new(false);
    };
    InGrid::new(ux < GRID_WIDTH && uy < GRID_HEIGHT)
}

/// Whether a storey index `z` is a valid level (`0..MAX_LEVELS`).
pub(super) fn z_in_grid(z: VoxelIndex) -> InGrid {
    InGrid::new(*z >= 0 && *z < i32::from(MAX_LEVELS))
}

/// Build the `(cell, level)` key for integer voxel coords — `z` is known in-range by
/// the caller (it is checked before this is called).
pub(super) fn key_of(x: VoxelIndex, y: VoxelIndex, z: VoxelIndex) -> CellLevel {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is checked in 0..MAX_LEVELS before this is called, so the u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(*z as u8);
    CellLevel::new(Cell::new(*x, *y), level)
}

/// Build a `(cell, level)` key, clamping the storey into `0..MAX_LEVELS` so an
/// out-of-grid `z` (an exit cell) still yields a representable key — the cell x/y is
/// recorded as-is (the key is a pure record here, never an index).
pub(super) fn key_of_clamped(x: VoxelIndex, y: VoxelIndex, z: VoxelIndex) -> CellLevel {
    let clamped = (*z).clamp(0, i32::from(MAX_LEVELS) - 1);
    key_of(x, y, VoxelIndex::new(clamped))
}

/// The sim-unit point along the ray at parameter `t` — `muzzle + t × dir`, built via
/// [`SimPos::new`] (AC #7: the impact point is a `SimPos` from `SimPos::new`).
pub(super) fn point_at(muzzle: SimPos, dir: MarchDir, t: RayParam) -> SimPos {
    let p = *muzzle + *dir * *t;
    SimPos::new(p.x, p.y, p.z)
}
