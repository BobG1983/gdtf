//! Relocated unit tests for the firing-arc containment helper (GTW-201 wave 22 — moved
//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    combatants::firing_arc::target_in_arc, ganger::Direction, metric::Cell, tuning::FiringArc,
};

// The actor's cell every case fires from — an arbitrary interior cell (the geometry
// is translation-invariant, so the absolute position is irrelevant).
const ACTOR: Cell = Cell::new(5, 5);

// GTW-242 — a target directly along the facing (0° off-axis) is always in-arc, for any
// positive arc width. East faces +x, so a target at greater x on the same row is dead
// ahead.
#[test]
fn on_axis_target_is_in_arc() {
    let arc = FiringArc::new(120.0);
    assert!(
        target_in_arc(Direction::East, ACTOR, Cell::new(8, 5), &arc),
        "a target dead ahead (0° off the facing) must be in-arc",
    );
}

// GTW-242 — a target 90° off the facing is OUTSIDE a 120° (±60°) arc. East faces +x; a
// target at greater y (South, +y in the −Y-North convention) is 90° off.
#[test]
fn ninety_degrees_off_is_out_of_arc_for_120() {
    let arc = FiringArc::new(120.0);
    assert!(
        !target_in_arc(Direction::East, ACTOR, Cell::new(5, 8), &arc),
        "a target 90° off the facing must be out of a 120° (±60°) arc",
    );
}

// GTW-242 boundary (AC4) — a target at EXACTLY arc/2 off the facing is IN-arc (the
// inclusive ≤ boundary). A 45° diagonal target is exactly arc/2 of a 90° arc; pinning
// the inclusive boundary in one assertion so a regression to `<` flips it.
#[test]
fn target_exactly_at_arc_edge_is_inclusive_in_arc() {
    // East faces +x; SouthEast (the +x,+y diagonal target) is exactly 45° off-axis.
    let edge = Cell::new(8, 8); // +3,+3 from ACTOR — a 45° diagonal
    // A 90° arc has a 45° half-angle — the diagonal sits EXACTLY on the edge.
    let arc_at_edge = FiringArc::new(90.0);
    assert!(
        target_in_arc(Direction::East, ACTOR, edge, &arc_at_edge),
        "a target at exactly arc/2 off the facing must be IN-arc (inclusive ≤ boundary)",
    );
    // One hair NARROWER than the diagonal's 45° must now exclude it — proving the
    // boundary is the discriminator, not a coincidence.
    let arc_just_inside = FiringArc::new(89.0);
    assert!(
        !target_in_arc(Direction::East, ACTOR, edge, &arc_just_inside),
        "a 45° target must be OUT of an 89° (<45° half-angle) arc",
    );
}

// GTW-242 AC6 — a co-located target (zero delta) is in-arc, with no NaN/panic from the
// angle computation (the degenerate zero-vector guard).
#[test]
fn co_located_target_is_in_arc_no_nan() {
    let arc = FiringArc::new(120.0);
    assert!(
        target_in_arc(Direction::North, ACTOR, ACTOR, &arc),
        "a co-located target (zero vector) must be in-arc (no turn, no NaN)",
    );
}

// GTW-242 AC5 — the arc is the discriminator: a fixed 90°-off-axis target flips from
// out-of-arc to in-arc as the arc widens past 180°, and a 0° target is in-arc even for
// a razor-thin arc. Proves the predicate reads the FiringArc, not a hardcoded width.
#[test]
fn arc_width_drives_containment() {
    let off_axis = Cell::new(5, 8); // 90° off East (South)
    // A wide arc (200° ⇒ ±100°) admits the 90°-off target.
    assert!(
        target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(200.0)),
        "a wide (200°) arc must admit a 90°-off target",
    );
    // A narrow arc (10° ⇒ ±5°) excludes it.
    assert!(
        !target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(10.0)),
        "a narrow (10°) arc must exclude a 90°-off target",
    );
    // Even a razor-thin arc admits a perfectly on-axis target.
    let on_axis = Cell::new(8, 5);
    assert!(
        target_in_arc(Direction::East, ACTOR, on_axis, &FiringArc::new(1.0)),
        "an on-axis target is in-arc even for a razor-thin arc",
    );
}
