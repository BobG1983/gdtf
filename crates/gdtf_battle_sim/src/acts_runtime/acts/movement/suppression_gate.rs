//! The GTW-537 pinned-movement legality gate — whether a
//! [`Suppressed`](crate::ganger::Suppressed) mover's chosen destination is legal
//! (strictly farther from the suppressor AND ending behind cover).

use crate::{
    cover::CoverLedger,
    ganger::Direction,
    metric::{Cell, CellLevel},
};

/// The ground-plane Chebyshev distance between two `(cell, level)` keys — `max(|dx|, |dy|)`
/// (GTW-537).
///
/// The sim's ESTABLISHED cell-distance metric (NOT a new one): the same `max(|dx|, |dy|)`
/// the AI's `chebyshev_xy` (`acts_runtime/ai/decide.rs`), the LOS engagement range gate, the
/// pathfinder heuristic, AND — decisively — the suppression producer's `within_radius`
/// (`acts_runtime/suppression/apply.rs`) all use, so "farther from the suppressor" agrees
/// with the disc suppression itself is measured on. The `z` storey is ignored (a ground
/// plane distance; suppression is a same-level effect this slice) — the suppressor anchor
/// and both the start and destination are on the mover's own storey by construction. A loop
/// magnitude (a comparison scalar, not a stored domain quantity), never a bare domain type.
fn chebyshev_xy(a: &CellLevel, b: &CellLevel) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    dx.max(dy)
}

/// Whether the cell one Moore-8 step from `dest` TOWARD `suppressor` holds registered cover
/// in the [`CoverLedger`] — the "ends behind cover relative to the suppressor" clause
/// (GTW-537).
///
/// Mirrors the GTW-526 auto-stance `cover_cell_toward` idiom
/// (`acts_runtime/suppression/stance.rs`): [`Direction::from_cells`] from `dest` toward the
/// `suppressor` cell picks the facing, [`Direction::cell_step`] the whole-cell delta, and the
/// stepped cell (kept on the destination's OWN storey — the cover a mover ducks behind is at
/// its level, not the suppressor's) is [`peek`](CoverLedger::peek)ed WITHOUT lazy seeding, so
/// a cell with no registered cover reads `None` = not behind cover. Returns `false` when the
/// destination and the suppressor share a ground cell (no direction — `from_cells` is `None`),
/// which is also not "farther", so such a destination is rejected on the distance clause too.
fn ends_behind_cover(dest: &CellLevel, suppressor: &CellLevel, cover: &CoverLedger) -> bool {
    // The canonical CellLevel accessors (GTW-565): the two ground cells and the
    // destination's own storey.
    let dest_cell = dest.cell();
    let suppressor_cell = suppressor.cell();
    let Some(dir) = Direction::from_cells(dest_cell, suppressor_cell) else {
        return false;
    };
    let step = dir.cell_step();
    let toward = Cell::new(dest_cell.x + step.x, dest_cell.y + step.y);
    cover.peek(&CellLevel::new(toward, dest.level())).is_some()
}

/// Whether a [`Suppressed`](crate::ganger::Suppressed) mover may legally step from `start` to `dest` (GTW-537) — the
/// HARD-REJECT movement-strictness gate.
///
/// A pinned unit's chosen destination is LEGAL only when BOTH clauses hold (F-movement
/// strictness — a HARD reject of an illegal destination, never a clamp):
///
/// 1. `dest` is STRICTLY FARTHER from the [`SuppressorCell`](crate::ganger::SuppressorCell)
///    than `start` — [`chebyshev_xy`]`(dest, suppressor) > `[`chebyshev_xy`]`(start,
///    suppressor)` (the sim's existing Chebyshev metric, the same disc suppression is
///    measured on); and
/// 2. `dest` ENDS BEHIND COVER relative to the suppressor — [`ends_behind_cover`].
///
/// Failing EITHER clause is illegal (the caller rejects with [`MoveRejection::Suppressed`](super::signals::MoveRejection::Suppressed),
/// no step). On a cover-sparse map both clauses can be unsatisfiable, pinning the unit — the
/// intended "pinned" feel (GTW-537 R1), NOT softened.
pub(super) fn suppressed_move_legal(
    start: &CellLevel,
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
) -> bool {
    let farther = chebyshev_xy(dest, suppressor) > chebyshev_xy(start, suppressor);
    farther && ends_behind_cover(dest, suppressor, cover)
}
