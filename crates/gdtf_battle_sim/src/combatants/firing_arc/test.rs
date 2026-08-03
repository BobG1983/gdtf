//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    combatants::firing_arc::target_in_arc, ganger::Direction, metric::Cell, tuning::FiringArc,
};

const ACTOR: Cell = Cell::new(5, 5);

#[test]
fn on_axis_target_is_in_arc() {
    let arc = FiringArc::new(120.0);
    assert!(
        *target_in_arc(Direction::East, ACTOR, Cell::new(8, 5), &arc),
        "a target dead ahead (0° off the facing) must be in-arc",
    );
}

#[test]
fn ninety_degrees_off_is_out_of_arc_for_120() {
    let arc = FiringArc::new(120.0);
    assert!(
        !*target_in_arc(Direction::East, ACTOR, Cell::new(5, 8), &arc),
        "a target 90° off the facing must be out of a 120° (±60°) arc",
    );
}

#[test]
fn target_exactly_at_arc_edge_is_inclusive_in_arc() {
    let edge = Cell::new(8, 8); 
    let arc_at_edge = FiringArc::new(90.0);
    assert!(
        *target_in_arc(Direction::East, ACTOR, edge, &arc_at_edge),
        "a target at exactly arc/2 off the facing must be IN-arc (inclusive ≤ boundary)",
    );
    let arc_just_inside = FiringArc::new(89.0);
    assert!(
        !*target_in_arc(Direction::East, ACTOR, edge, &arc_just_inside),
        "a 45° target must be OUT of an 89° (<45° half-angle) arc",
    );
}

#[test]
fn co_located_target_is_in_arc_no_nan() {
    let arc = FiringArc::new(120.0);
    assert!(
        *target_in_arc(Direction::North, ACTOR, ACTOR, &arc),
        "a co-located target (zero vector) must be in-arc (no turn, no NaN)",
    );
}

#[test]
fn arc_width_drives_containment() {
    let off_axis = Cell::new(5, 8); 
    assert!(
        *target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(200.0)),
        "a wide (200°) arc must admit a 90°-off target",
    );
    assert!(
        !*target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(10.0)),
        "a narrow (10°) arc must exclude a 90°-off target",
    );
    let on_axis = Cell::new(8, 5);
    assert!(
        *target_in_arc(Direction::East, ACTOR, on_axis, &FiringArc::new(1.0)),
        "an on-axis target is in-arc even for a razor-thin arc",
    );
}
