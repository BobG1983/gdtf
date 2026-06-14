//! Battle-space metric: the unified px coordinate system the shot pipeline flies
//! in (see `docs/combat/battle-space.md`).
//!
//! **One metric, px on all three axes**: x/y are ground-plane px, z is height
//! px — the *same* axis, so a 3D shot vector's angles are real. A cell is
//! `floor(pos_px / CELL_PITCH_PX)`; a z-level is `floor(z_px / Z_LEVEL_HEIGHT)`.
//! The view's projection never enters the model — these are battle-space px;
//! projection is the presenter's job (`gdtf_battle_presenter`).
//!
//! Positions are math vectors in this metric (Bevy re-exports glam as
//! `bevy::math`, so these are bevy's own vector types). The newtypes below name
//! each domain quantity so a cell can never be mistaken for a world point and a
//! storey index can never be passed where a coordinate belongs.

use bevy::{
    math::{IVec2, IVec3, Vec3},
    prelude::Deref,
};

/// One grid cell's pitch on the x/y ground plane, in battle-space px.
///
/// Derived from the source tile *width* (180×104) so the pre-existing z-px
/// datums keep their proportions on the shared axis (a standing ganger, 175 px,
/// ≈ one cell pitch). An `f32` because it divides continuous px positions into
/// cells (`floor(pos_px / CELL_PITCH_PX)`) — see `docs/combat/battle-space.md`.
pub const CELL_PITCH_PX: f32 = 180.0;

/// One discrete storey of the 60×60×8 coarse grid, in battle-space px.
///
/// The 1:1 vertical projection datum: a model point on storey `k`'s floor sits
/// at `z = k × Z_LEVEL_HEIGHT` px, and the presenter's per-level lift equals
/// this, so trajectories and floors can never drift. An `f32` for the same
/// continuous-division reason as [`CELL_PITCH_PX`]. Retuned 200 → 170 (user
/// ruling 2026-06-11) — see `docs/combat/battle-space.md`.
pub const Z_LEVEL_HEIGHT: f32 = 170.0;

/// Number of storeys in the coarse grid — valid [`Level`] values are
/// `0..MAX_LEVELS`.
///
/// A `u8` because the coarse grid is 60×60×**8**: a storey index is a tiny
/// non-negative integer, and the type bounds it cheaply. See
/// `docs/combat/battle-space.md` (the 60×60×8 coarse grid).
pub const MAX_LEVELS: u8 = 8;

/// A 2D grid coordinate — a cell on the ground plane, in cell units.
///
/// Gameplay reasons in cells: `floor(pos_px / CELL_PITCH_PX)` on each ground
/// axis. Wraps `IVec2` (not a world point) so a cell can never be passed where
/// battle-space px ([`BattlePx`]) is meant. Negative components bucket
/// correctly because the conversion floors, never rounds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell(IVec2);

/// A 0-based storey index — which floor of the coarse grid, valid `0..`[`MAX_LEVELS`].
///
/// Distinct from a raw coordinate axis: it indexes storeys, and its world lift
/// is `level × Z_LEVEL_HEIGHT` px. Wraps `u8` so a storey can never be confused
/// with a cell coordinate or a px height.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Level(u8);

/// The canonical 3D grid key: `(cell.x, cell.y, level)`.
///
/// The single (cell, storey) identity used to key the coarse occupancy and the
/// cover ledger. Wraps `IVec3` — its `z` is a *storey index*, NOT a px height,
/// which is what distinguishes it from [`BattlePx`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellLevel(IVec3);

/// A world-space position in battle-space px — the unified px metric.
///
/// x/y are ground-plane px and z is height px on the *same* axis, so a vector
/// of these has honest 3D angles (the whole reason for one metric). Wraps
/// `Vec3` so a continuous px point can never be confused with a discrete
/// [`Cell`]/[`CellLevel`] key.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct BattlePx(Vec3);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_consts_match_battle_space_doc() {
        // Pinned to docs/combat/battle-space.md: cell_pitch_px = 180,
        // z_level_height = 170, and the 60×60×8 grid → 8 levels. Compared by
        // bit pattern: these are exactly representable, and `to_bits` is an
        // exact integer equality (no `float_cmp` lint, no epsilon needed).
        assert_eq!(CELL_PITCH_PX.to_bits(), 180.0_f32.to_bits());
        assert_eq!(Z_LEVEL_HEIGHT.to_bits(), 170.0_f32.to_bits());
        assert_eq!(MAX_LEVELS, 8);
    }

    #[test]
    fn cell_wraps_ivec2_and_derefs() {
        let cell = Cell(IVec2::new(3, -4));
        // Deref reaches the inner IVec2's fields.
        assert_eq!(cell.x, 3);
        assert_eq!(cell.y, -4);
        assert_eq!(*cell, IVec2::new(3, -4));
    }

    #[test]
    fn level_wraps_u8_and_derefs() {
        let level = Level(5);
        // Deref reaches the inner u8.
        assert_eq!(*level, 5u8);
        assert!(*level < MAX_LEVELS);
    }

    #[test]
    fn cell_level_wraps_ivec3_and_derefs() {
        let key = CellLevel(IVec3::new(7, 8, 2));
        assert_eq!(key.x, 7);
        assert_eq!(key.y, 8);
        // z is the storey index, not a px height.
        assert_eq!(key.z, 2);
        assert_eq!(*key, IVec3::new(7, 8, 2));
    }

    #[test]
    fn battle_px_wraps_vec3_and_derefs() {
        let p = BattlePx(Vec3::new(180.0, 360.0, 170.0));
        // Deref reaches the inner Vec3 (its `glam` PartialEq compares the whole
        // vector, so this checks the inner type without raw `f32 ==`).
        assert_eq!(*p, Vec3::new(180.0, 360.0, 170.0));
        // Per-axis pin by bit pattern (exactly representable px datums).
        assert_eq!(p.x.to_bits(), 180.0_f32.to_bits());
        assert_eq!(p.y.to_bits(), 360.0_f32.to_bits());
        assert_eq!(p.z.to_bits(), 170.0_f32.to_bits());
    }
}
