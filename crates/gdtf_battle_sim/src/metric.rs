//! Battle-space metric: the cubic-voxel coordinate system the shot pipeline
//! flies in (see `docs/combat/battle-space.md`).
//!
//! **One continuous cubic-voxel metric, no pixels anywhere in the model**: x/y
//! are ground-plane cells, z is height in *levels*, and **one sim unit is one
//! cell on x = one cell on y = one level on z** — cubic voxels over the 60×60×8
//! grid, so a 3D shot vector's angles are honest. A cell is `floor(pos.x)` /
//! `floor(pos.y)`; a z-level is `floor(pos.z)`. The view's pixel projection
//! never enters the model — these are sim-unit voxel coordinates; that
//! projection is the presenter's job (`gdtf_battle_presenter`).
//!
//! Positions are math vectors in this metric (Bevy re-exports glam as
//! `bevy::math`, so these are bevy's own vector types). The newtypes below name
//! each domain quantity so a cell can never be mistaken for a sim point and a
//! storey index can never be passed where a coordinate belongs.

use bevy::{
    math::{IVec2, IVec3, Vec3},
    prelude::Deref,
};

/// Number of storeys in the coarse grid — valid [`Level`] values are
/// `0..MAX_LEVELS`.
///
/// A `u8` because the coarse grid is 60×60×**8**: a storey index is a tiny
/// non-negative integer, and the type bounds it cheaply. See
/// `docs/combat/battle-space.md` (the 60×60×8 coarse grid).
pub const MAX_LEVELS: u8 = 8;

/// A 2D grid coordinate — a cell on the ground plane, in cell units.
///
/// Gameplay reasons in cells: `floor(pos.x)` / `floor(pos.y)` on each ground
/// axis (one cell = 1.0 sim unit). Wraps `IVec2` (not a sim point) so a cell can
/// never be passed where a sim-unit position ([`SimPos`]) is meant. Negative
/// components bucket correctly because [`pos_to_cell`] floors, never rounds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell(IVec2);

impl Cell {
    /// Build a cell from its ground-plane `x`/`y` grid coordinates (cell units).
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }
}

/// A 0-based storey index — which floor of the coarse grid, valid `0..`[`MAX_LEVELS`].
///
/// Distinct from a raw coordinate axis: it indexes storeys (each one level = 1.0
/// sim unit on z). Wraps `u8` so a storey can never be confused with a cell
/// coordinate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    /// Build a storey index. Callers are expected to keep it in `0..`[`MAX_LEVELS`].
    #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// The canonical 3D grid key: `(cell.x, cell.y, level)`.
///
/// The single (cell, storey) identity used to key the coarse occupancy and the
/// cover ledger. Wraps `IVec3` — its `z` is a *storey index*, NOT a continuous
/// height, which is what distinguishes it from [`SimPos`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellLevel(IVec3);

impl CellLevel {
    /// Build a `(cell, level)` key from a ground-plane [`Cell`] and a storey
    /// [`Level`].
    ///
    /// Composes the two typed coordinates into the canonical `IVec3` key
    /// (`x`/`y` from the cell, `z` = the storey index) — the one constructor for
    /// the grid identity, keeping the inner `IVec3` private so callers can never
    /// hand-build a `z` that is a continuous height instead of a storey.
    #[must_use]
    pub fn new(cell: Cell, level: Level) -> Self {
        Self(IVec3::new(cell.x, cell.y, i32::from(*level)))
    }
}

/// A continuous position in the cubic-voxel metric — sim units on every axis.
///
/// x/y are ground-plane cell units and z is height in level units, all the
/// *same* unit (one cell-width = one level-height = 1.0), so a vector of these
/// has honest 3D angles (the whole reason for cubic voxels). Wraps `Vec3` so a
/// continuous sim point can never be confused with a discrete
/// [`Cell`]/[`CellLevel`] key. Build one with [`SimPos::new`]; [`cell_center`]
/// and [`pos_to_cell`] are the sim-unit conversions to and from the integer
/// grid.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct SimPos(Vec3);

impl SimPos {
    /// Build a sim-unit position from its `x`/`y` (cell units) and `z` (level
    /// units) components — the public constructor for a cubic-voxel point.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
}

/// The sim-unit position of a `(cell, level)`'s center on its level's floor.
///
/// The center of a cell is offset half a cell (0.5 sim units) on x and y from
/// its integer corner; z is the level's floor (`level` sim units exactly, the
/// 0.0 level-fraction). Inverse of [`pos_to_cell`] on the cell/level it names
/// (`pos_to_cell(cell_center(c, l)) == (c, l)`). See `docs/combat/battle-space.md`.
#[must_use]
pub fn cell_center(cell: Cell, level: Level) -> SimPos {
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60 / 0..8); the f32 conversion is exact for this range"
    )]
    SimPos::new(cell.x as f32 + 0.5, cell.y as f32 + 0.5, f32::from(*level))
}

/// The `(cell, level)` a sim-unit position falls in — **floored**, never
/// rounded.
///
/// `cell = (floor(pos.x), floor(pos.y))` and `level = floor(pos.z)`, so negative
/// coordinates bucket into the correct (lower) cell rather than toward zero. The
/// returned [`Level`] is clamped non-negative (a sub-floor `z` buckets to level
/// 0) so it fits the `u8` storey type; callers keep it in `0..`[`MAX_LEVELS`].
/// See `docs/combat/battle-space.md` ("`pos_to_cell` floors — never rounds").
#[must_use]
pub fn pos_to_cell(pos: SimPos) -> (Cell, Level) {
    let cell = Cell::new(floor_to_i32(pos.x), floor_to_i32(pos.y));
    let storey = floor_to_i32(pos.z).clamp(0, i32::from(u8::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=u8::MAX above, so this u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(storey as u8);
    (cell, level)
}

/// Floor a sim-unit `f32` coordinate to its integer cell/level index.
///
/// Uses [`f32::floor`] so negative coordinates go to the lower integer (−0.5 →
/// −1), never toward zero — the floor-not-round contract of the metric. The
/// result is clamped into the `i32` range so a wild out-of-grid coordinate can
/// never wrap on the cast.
const fn floor_to_i32(coord: f32) -> i32 {
    let floored = coord.floor();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after floor"
    )]
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    clamped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_consts_match_battle_space_doc() {
        // Pinned to docs/combat/battle-space.md: the 60×60×8 grid → 8 levels.
        // This is a coordinate-system FACT (not tunable), so it stays pinned —
        // unlike the band-edge level-fractions, which are tuning data.
        assert_eq!(MAX_LEVELS, 8);
    }

    #[test]
    fn cell_wraps_ivec2_and_derefs() {
        let cell = Cell::new(3, -4);
        // Deref reaches the inner IVec2's fields.
        assert_eq!(cell.x, 3);
        assert_eq!(cell.y, -4);
        assert_eq!(*cell, IVec2::new(3, -4));
    }

    #[test]
    fn level_wraps_u8_and_derefs() {
        let level = Level::new(5);
        // Deref reaches the inner u8.
        assert_eq!(*level, 5u8);
        assert!(*level < MAX_LEVELS);
    }

    #[test]
    fn cell_level_wraps_ivec3_and_derefs() {
        let key = CellLevel::new(Cell::new(7, 8), Level::new(2));
        assert_eq!(key.x, 7);
        assert_eq!(key.y, 8);
        // z is the storey index, not a continuous height.
        assert_eq!(key.z, 2);
        assert_eq!(*key, IVec3::new(7, 8, 2));
    }

    #[test]
    fn sim_pos_wraps_vec3_and_derefs() {
        let p = SimPos::new(1.5, 2.5, 3.0);
        // Deref reaches the inner Vec3 (its `glam` PartialEq compares the whole
        // vector, so this checks the inner type without raw `f32 ==`).
        assert_eq!(*p, Vec3::new(1.5, 2.5, 3.0));
        // Per-axis pin by bit pattern (exactly representable sim-unit values).
        assert_eq!(p.x.to_bits(), 1.5_f32.to_bits());
        assert_eq!(p.y.to_bits(), 2.5_f32.to_bits());
        assert_eq!(p.z.to_bits(), 3.0_f32.to_bits());
    }

    #[test]
    fn cell_center_sits_at_half_cell_offset_on_level_floor() {
        // The center of cell (4, 7) on level 2 is half a cell in on x/y and on
        // the level-2 floor (z = 2.0 exactly, the 0.0 level-fraction).
        let center = cell_center(Cell::new(4, 7), Level::new(2));
        assert_eq!(center.x.to_bits(), 4.5_f32.to_bits());
        assert_eq!(center.y.to_bits(), 7.5_f32.to_bits());
        assert_eq!(center.z.to_bits(), 2.0_f32.to_bits());
    }

    #[test]
    fn cell_center_pos_to_cell_round_trip() {
        // cell_center → pos_to_cell is the identity on the (cell, level) it
        // names, for a positive cell.
        let cell = Cell::new(3, 9);
        let level = Level::new(4);
        let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
        assert_eq!(back_cell, cell);
        assert_eq!(back_level, level);
    }

    #[test]
    fn pos_to_cell_floors_negative_coordinates() {
        // The floor-not-round contract: a NEGATIVE-coordinate cell must bucket
        // to the lower integer, never toward zero. cell_center(-2, -3) sits at
        // (-1.5, -2.5), which floors back to (-2, -3) — a round would give
        // (-1, -2) or (-2, -2) depending on the half, which is wrong.
        let cell = Cell::new(-2, -3);
        let level = Level::new(0);
        let (back_cell, back_level) = pos_to_cell(cell_center(cell, level));
        assert_eq!(back_cell, cell, "negative cell must floor, not round");
        assert_eq!(back_level, level);

        // A point anywhere inside a negative cell floors to that cell's corner.
        let (mid_cell, _) = pos_to_cell(SimPos::new(-0.1, -0.9, 0.0));
        assert_eq!(
            mid_cell,
            Cell::new(-1, -1),
            "a fractional negative coordinate floors to the lower cell",
        );
    }

    #[test]
    fn pos_to_cell_levels_floor_within_a_storey() {
        // z within a storey floors to that storey: 2.0..3.0 → level 2.
        let (_, low_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.0));
        let (_, high_in_storey) = pos_to_cell(SimPos::new(0.5, 0.5, 2.99));
        assert_eq!(low_in_storey, Level::new(2));
        assert_eq!(high_in_storey, Level::new(2));
    }
}
