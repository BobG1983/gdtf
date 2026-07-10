//! The GTW-390/391 stair surface: the [`StairEyeOffset`] eye z-lift for observers
//! on authored stair tiles, plus dual-cell stair presence registration/teardown.

use bevy::prelude::Deref;

use super::storage::OccupancyGrid;
use crate::{
    cover::HeightBand,
    metric::{CellLevel, Level, MAX_LEVELS},
};

/// The **eye z-lift** applied to an observer standing on an authored stair tile
/// (GTW-390).
///
/// A stair tile sits at the junction between storeys: the observer's body spans
/// two levels, so their eye is higher than a flat-floor observer at the same
/// `(cell, level)`. This offset is ADDED to `f32::from(level) +
/// muzzle_height(stance)` in the `eye_anchor` probe function (GTW-390) to reflect
/// that extra height — **stance-gated at the use site** (Prone → 0.0, any upright
/// posture → +0.5).
///
/// Non-stair cells are simply absent from the
/// [`OccupancyGrid`]'s stair-cell set, so
/// [`OccupancyGrid::stair_eye_offset_at`] returns `StairEyeOffset(0.0)` by default
/// (i.e. `Default`). Private inner + derived [`Deref`] (no-bare-types house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct StairEyeOffset(f32);

impl StairEyeOffset {
    /// Build a stair eye-offset from the raw level-fraction `offset`.
    ///
    /// Pass `0.0` for a non-stair cell or a Prone observer (no lift); pass `0.5`
    /// for a Standing or Crouching observer on an authored stair tile.
    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

/// Whether a `(cell, level)` is an **authored stair tile** — the membership answer
/// [`OccupancyGrid::is_stair_cell`] returns (the cell is a registered stair endpoint).
///
/// A named newtype over `bool` (no-bare-types: stair-tile membership is a domain fact, not a
/// bare boolean — a stair endpoint reads `StairCell(true)`). `true` when the cell was
/// registered via [`mark_stair_cell`](OccupancyGrid::mark_stair_cell) at battle setup, which
/// the LOS probe reads to lift an observer's eye z. Private inner + derived [`Deref`] (house
/// style).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StairCell(bool);

impl StairCell {
    /// Build a stair-tile membership answer from its boolean state.
    #[must_use]
    pub const fn new(is_stair: bool) -> Self {
        Self(is_stair)
    }
}

impl OccupancyGrid {
    /// Mark `cell_level` as an **authored stair tile** (GTW-390) — insert it into the
    /// [`stair_cells`](OccupancyGrid::is_stair_cell) set so the LOS probe can lift
    /// the eye z for an observer standing on a stair endpoint.
    ///
    /// Called at battle setup for every endpoint of every
    /// [`Stair`](crate::vertical::LinkKind::Stair) link in the situation's
    /// vertical-link list ([`Ladder`](crate::vertical::LinkKind::Ladder) endpoints
    /// are excluded — ladders do not share the half-level stair eye-lift). Re-marking
    /// the same cell is a harmless no-op (set semantics).
    pub fn mark_stair_cell(&mut self, cell_level: CellLevel) {
        self.stair_cells.insert(cell_level);
    }

    /// Whether `cell_level` is an authored stair tile — a read-only membership test.
    ///
    /// Returns `true` when the cell was registered via
    /// [`mark_stair_cell`](OccupancyGrid::mark_stair_cell) at battle setup (it is a
    /// stair endpoint). A cell absent from the set — including every cell on a grid
    /// built with `OccupancyGrid::new()` — returns `false` (non-stair = no eye-lift).
    #[must_use]
    pub fn is_stair_cell(&self, cell_level: &CellLevel) -> StairCell {
        StairCell::new(self.stair_cells.contains(cell_level))
    }

    /// The raw [`StairEyeOffset`] for `cell_level` — `StairEyeOffset(0.5)` when the
    /// cell is an authored stair tile, `StairEyeOffset(0.0)` otherwise (non-stair or
    /// out-of-range).
    ///
    /// This is the **un-gated** offset: the caller (`eye_anchor` in
    /// [`crate::los`]) applies the **stance gate**
    /// (`Prone → 0.0`, any upright posture → read this value) before adding it to the
    /// eye z. Returning `0.5` for every non-Prone stair observer and `0.0` for every
    /// other combination keeps the gate logic concentrated at the `eye_anchor` call
    /// site.
    #[must_use]
    pub fn stair_eye_offset_at(&self, cell_level: &CellLevel) -> StairEyeOffset {
        if self.stair_cells.contains(cell_level) {
            StairEyeOffset::new(0.5)
        } else {
            StairEyeOffset::new(0.0)
        }
    }

    /// Register a ganger's **stair presence** atomically — write the lower cell and,
    /// when safe, the upper cell — returning the upper [`CellLevel`] written so the
    /// caller can record it in [`PrevSlot`](crate::occupancy_sync::PrevSlot) for
    /// verbatim teardown (GTW-391).
    ///
    /// Writes the lower cell unconditionally: `set_occupant(lower, Some(entity))` +
    /// `set_occupant_band(lower, Some(lower_band))`.
    ///
    /// Then attempts the upper cell `(cell, level+1)`:
    ///
    /// * Returns `None` (lower-only) when there is no upper cell (top storey —
    ///   `level == MAX_LEVELS - 1`).
    /// * Returns `None` (lower-only) when the upper cell is already occupied by a
    ///   **different** entity — the **occupancy guard** (Blocker 3): the single-occupant
    ///   slot must not be stomped. A ganger at `(cell, level+1)` as its own lower cell
    ///   would lose its presence if we overwrote its slot here.
    /// * Writes `set_occupant(upper, Some(entity))` + `set_occupant_band(upper,
    ///   Some(HeightBand::Low))` and returns `Some(upper)` when the upper cell is
    ///   unoccupied, OR already owned by the SAME entity (idempotent re-register: a
    ///   ganger re-posing upright on the same stair just refreshes the band).
    ///
    /// The caller ([`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) /
    /// [`build_from_occupancy_input`](OccupancyGrid::build_from_occupancy_input)) stores
    /// the returned `Option<CellLevel>` in `PrevSlot` so every teardown path can replay
    /// the exact written set without re-derivation.
    pub fn register_stair_presence(
        &mut self,
        lower: CellLevel,
        entity: bevy::prelude::Entity,
        lower_band: HeightBand,
    ) -> Option<CellLevel> {
        // Always write the lower cell.
        self.set_occupant(lower, Some(entity));
        self.set_occupant_band(lower, Some(lower_band));

        // Compute the cell directly above — None at the top storey.
        let upper = Self::upper_cell(&lower)?;

        // OCCUPANCY GUARD (Blocker 3): never stomp a cell owned by a different entity.
        match self.occupant(&upper) {
            None => {
                // Upper cell is free — claim it.
                self.set_occupant(upper, Some(entity));
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(other) if other == entity => {
                // Idempotent re-register: same entity already owns the upper cell.
                // Refresh the band (e.g. upright re-pose after a stance change).
                self.set_occupant_band(upper, Some(HeightBand::Low));
                Some(upper)
            }
            Some(_) => {
                // Upper cell owned by a different entity — lower-only (no stomp).
                None
            }
        }
    }

    /// Clear `entity`'s upper-cell stair presence at `upper` (occupant + band),
    /// guarded by ownership so it never stomps a slot another entity now owns (GTW-391).
    ///
    /// Only clears when `occupant(&upper) == Some(entity)` — if the cell was taken
    /// over by a different entity in the meantime, this is a harmless no-op. Used by
    /// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) and
    /// [`sync_dead_gangers`](crate::occupancy_sync::sync_dead_gangers) to replay the
    /// upper-cell teardown recorded in `PrevSlot::upper`.
    pub fn clear_stair_upper(&mut self, upper: CellLevel, entity: bevy::prelude::Entity) {
        if self.occupant(&upper) == Some(entity) {
            self.set_occupant(upper, None);
            self.set_occupant_band(upper, None);
        }
    }

    /// The `(cell, level+1)` cell directly above `lower`, or `None` when `lower` is at
    /// the top storey (`level == MAX_LEVELS - 1`) — the upper-presence target for a
    /// stair occupant (GTW-391).
    ///
    /// Private: only [`register_stair_presence`](OccupancyGrid::register_stair_presence)
    /// calls this. The arithmetic is `level + 1` checked against `MAX_LEVELS`.
    fn upper_cell(lower: &CellLevel) -> Option<CellLevel> {
        // The storey via the canonical `CellLevel::level` accessor (GTW-565), then a
        // checked u8 add against the ceiling.
        let next = (*lower.level()).checked_add(1)?;
        if next >= MAX_LEVELS {
            return None;
        }
        Some(CellLevel::new(lower.cell(), Level::new(next)))
    }
}
