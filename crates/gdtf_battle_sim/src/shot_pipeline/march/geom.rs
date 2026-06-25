//! Stateless grid + ray geometry helpers shared by the DDA walk — the grid-bounds
//! tests, the `(cell, level)` key builders, and the point-along-the-ray evaluator
//! (sim units; one cell on x = one cell on y = one level on z = 1.0).

use bevy::math::Vec3;

use crate::{
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
};

/// Whether `(x, y)` is inside the ground-plane grid extent.
pub(super) fn xy_in_grid(x: i32, y: i32) -> bool {
    let Ok(ux) = usize::try_from(x) else {
        return false;
    };
    let Ok(uy) = usize::try_from(y) else {
        return false;
    };
    ux < GRID_WIDTH && uy < GRID_HEIGHT
}

/// Whether a storey index `z` is a valid level (`0..MAX_LEVELS`).
pub(super) fn z_in_grid(z: i32) -> bool {
    z >= 0 && z < i32::from(MAX_LEVELS)
}

/// Build the `(cell, level)` key for integer voxel coords — `z` is known in-range by
/// the caller (it is checked before this is called).
pub(super) fn key_of(x: i32, y: i32, z: i32) -> CellLevel {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is checked in 0..MAX_LEVELS before this is called, so the u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(z as u8);
    CellLevel::new(Cell::new(x, y), level)
}

/// Build a `(cell, level)` key, clamping the storey into `0..MAX_LEVELS` so an
/// out-of-grid `z` (an exit cell) still yields a representable key — the cell x/y is
/// recorded as-is (the key is a pure record here, never an index).
pub(super) fn key_of_clamped(x: i32, y: i32, z: i32) -> CellLevel {
    let clamped = z.clamp(0, i32::from(MAX_LEVELS) - 1);
    key_of(x, y, clamped)
}

/// The sim-unit point along the ray at parameter `t` — `muzzle + t × dir`, built via
/// [`SimPos::new`] (AC #7: the impact point is a `SimPos` from `SimPos::new`).
pub(super) fn point_at(muzzle: SimPos, dir: Vec3, t: f32) -> SimPos {
    let p = *muzzle + dir * t;
    SimPos::new(p.x, p.y, p.z)
}
