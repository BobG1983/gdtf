//! The cubic-voxel coordinate types + conversions — [`Cell`] / [`Level`] /
//! [`CellLevel`] / [`SimPos`] and their RON authoring shapes, plus [`cell_center`] /
//! [`pos_to_cell`]. See the module docs (`super`) for the sim-unit metric.

use bevy::{
    math::{IVec2, IVec3, Vec3},
    prelude::Deref,
};
use serde::{Deserialize, Serialize};

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
///
/// Deserializes through an `(x, y)` authoring shape ([`CellDef`]) that routes the
/// pair through [`Cell::new`] — so authored RON writes `(x: .., y: ..)` rather
/// than reaching the inner glam `IVec2` directly.
///
/// Serializes through the SAME [`CellDef`] shape (`#[serde(into = "CellDef")]`), so a
/// written cell round-trips byte-for-byte back through the `from = "CellDef"` reader
/// (the editor prefab saver — GTW-432). A *derived* `Serialize` would instead emit the
/// raw private `IVec2`, which the `(x, y)` reader could not parse — the `into`/`from`
/// pair keeps both directions on the one authoring shape.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "CellDef", into = "CellDef")]
pub struct Cell(IVec2);

impl Cell {
    /// Build a cell from its ground-plane `x`/`y` grid coordinates (cell units).
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }
}

/// The authored RON shape a [`Cell`] deserializes from — its `x`/`y` grid
/// coordinates as a named pair, routed through [`Cell::new`] (never the raw
/// `IVec2`).
///
/// A serde intermediate (`#[serde(from = "CellDef", into = "CellDef")]` on [`Cell`]) so
/// an authored situation writes a cell as `(x: 5, y: 6)` and the value flows through the
/// typed constructor on read AND back out through the same shape on write — keeping the
/// inner `IVec2` private and the no-bare-types contract intact in both directions.
#[derive(Deserialize, Serialize)]
pub struct CellDef {
    /// The cell's ground-plane `x` grid coordinate (cell units).
    x: i32,
    /// The cell's ground-plane `y` grid coordinate (cell units).
    y: i32,
}

impl From<CellDef> for Cell {
    fn from(def: CellDef) -> Self {
        Self::new(def.x, def.y)
    }
}

impl From<Cell> for CellDef {
    fn from(cell: Cell) -> Self {
        Self {
            x: cell.x,
            y: cell.y,
        }
    }
}

/// A 0-based storey index — which floor of the coarse grid, valid `0..`[`MAX_LEVELS`].
///
/// Distinct from a raw coordinate axis: it indexes storeys (each one level = 1.0
/// sim unit on z). Wraps `u8` so a storey can never be confused with a cell
/// coordinate. `#[serde(transparent)]` lets an authored RON storey index parse as
/// a bare integer (the tuning-leaf precedent).
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
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
///
/// Deserializes through a `(cell, level)` authoring shape ([`CellLevelDef`]) that
/// routes the pair through [`CellLevel::new`] — so authored RON names the typed
/// [`Cell`] + [`Level`] and the storey-index `z` is always *constructed*, never a
/// free continuous height an author could write into the raw `IVec3`. This is the
/// invariant `#[serde(from = "CellLevelDef")]` protects (unlike [`SimPos`], which
/// is deliberately NOT `Deserialize` — its continuous `z` must never be authorable
/// as a storey).
///
/// Serializes through the SAME [`CellLevelDef`] shape (`#[serde(into = "CellLevelDef")]`)
/// — its `z` storey index written back as a typed [`Cell`] + [`Level`] pair, so a written
/// `(cell, level)` round-trips byte-for-byte through the `from = "CellLevelDef"` reader
/// (the editor prefab saver — GTW-432). A *derived* `Serialize` would emit the raw private
/// `IVec3`, which the reader could not parse.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "CellLevelDef", into = "CellLevelDef")]
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

    /// The ground-plane [`Cell`] of this key — its `x`/`y`, the storey `z`
    /// dropped.
    ///
    /// The inverse of [`CellLevel::new`]'s cell half: recovering the typed
    /// [`Cell`] lives HERE, with the family type, so call sites never hand-roll
    /// `Cell::new(key.x, key.y)` (GTW-565 — the decompose belongs beside the
    /// compose).
    #[must_use]
    pub const fn cell(&self) -> Cell {
        Cell::new(self.0.x, self.0.y)
    }

    /// The storey [`Level`] of this key — its `z` narrowed back to the
    /// [`Level`]'s `u8`.
    ///
    /// This is the ONE place in the workspace a key's storey-index `z` (`i32`
    /// in the inner `IVec3`) narrows to `u8` (GTW-565). Every constructed key's
    /// `z` came from a [`Level`] via [`CellLevel::new`], so it is already in
    /// `0..=u8::MAX` and the clamp is a no-op on every reachable key; an
    /// out-of-range `z` (impossible by construction) clamps into range rather
    /// than panicking (the no-panic contract). The continuous-height narrow in
    /// [`pos_to_cell`] converts a DIFFERENT value (a floored `f32` height) and
    /// deliberately stays separate.
    #[must_use]
    pub fn level(&self) -> Level {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "z is a storey index constructed from a Level's u8; clamped to \
                      0..=u8::MAX here so the cast cannot wrap or sign-flip"
        )]
        let storey = self.0.z.clamp(0, i32::from(u8::MAX)) as u8;
        Level::new(storey)
    }

    /// Decompose this key into its `(`[`Cell`]`, `[`Level`]`)` pair — the
    /// exact inverse of [`CellLevel::new`] for every constructed key.
    ///
    /// The one-call decompose for the many call sites that need both halves
    /// (GTW-565 replaced six hand-rolled per-crate copies with this); the
    /// storey narrow rides [`CellLevel::level`], the single canonical clamp.
    #[must_use]
    pub fn split(&self) -> (Cell, Level) {
        (self.cell(), self.level())
    }
}

/// The authored RON shape a [`CellLevel`] deserializes from — a typed [`Cell`]
/// plus a typed [`Level`], routed through [`CellLevel::new`].
///
/// A serde intermediate (`#[serde(from = "CellLevelDef", into = "CellLevelDef")]` on
/// [`CellLevel`]) so an authored situation names the `(cell, level)` pair and the value
/// flows through the typed constructor on read AND back out through the same shape on
/// write. This is the AC2 guarantee: the storey-index `z` is always *constructed* from a
/// [`Level`], never authored as a raw continuous height in the inner `IVec3` — so the
/// no-bare-types / storey-not-height invariant survives both deserialization and
/// serialization (the GTW-432 editor saver round-trips through this shape).
#[derive(Deserialize, Serialize)]
pub struct CellLevelDef {
    /// The ground-plane cell of the key.
    cell:  Cell,
    /// The storey index of the key.
    level: Level,
}

impl From<CellLevelDef> for CellLevel {
    fn from(def: CellLevelDef) -> Self {
        Self::new(def.cell, def.level)
    }
}

impl From<CellLevel> for CellLevelDef {
    fn from(key: CellLevel) -> Self {
        // Recover the typed `(Cell, Level)` pair through the key's own accessors
        // (GTW-565): the serde write rides the SAME decompose (and the same
        // canonical clamp, inside `CellLevel::level`) every caller uses, so the
        // GTW-432 editor-saver round-trip and the call sites can never drift.
        let (cell, level) = key.split();
        Self { cell, level }
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
