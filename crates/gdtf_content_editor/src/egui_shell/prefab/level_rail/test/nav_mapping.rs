//! A1 — the click→level mapping: row order (top storey first), the clamped jump, and
//! the wheel-scrub fold.

use gdtf_battle_sim::metric::Level;

use super::support::size;
use crate::{
    canvas::CurrentEditLevel,
    egui_shell::prefab::level_rail::{rail_ui::rail_rows_top_first, scrub::ScrubAccumulator},
};

/// A row click routes through the EXISTING clamp: an in-range storey lands exactly; an
/// over-extent target saturates at the top storey (never outside the drawable volume).
#[test]
fn click_jump_routes_through_the_kept_clamp() {
    let size = size(); // 3 storeys → valid indices 0..=2.
    let jumped = CurrentEditLevel::jumped(Level::new(1), size);
    assert_eq!(*jumped.level(), 1, "an in-range click lands on its storey");
    let clamped = CurrentEditLevel::jumped(Level::new(7), size);
    assert_eq!(
        *clamped.level(),
        2,
        "an over-extent target clamps to levels-1 (the kept clamp)"
    );
}

/// The rail lists the TOP storey first (C1: ceiling at the top), so row order maps to
/// descending storey indices.
#[test]
fn rows_are_top_storey_first() {
    let rows: Vec<u8> = rail_rows_top_first(3).map(|level| *level).collect();
    assert_eq!(rows, vec![2, 1, 0], "top storey first, ground last");
}

/// The wheel-scrub fold yields whole storey steps and carries the fractional remainder;
/// a zero-point pass (the egui multipass re-run) yields no step.
#[test]
fn wheel_fold_accumulates_whole_steps() {
    let mut scrub = ScrubAccumulator::default();
    assert_eq!(scrub.fold(25.0), 0, "half a step folds to nothing yet");
    assert_eq!(
        scrub.fold(25.0),
        1,
        "the carried remainder completes a step"
    );
    assert_eq!(scrub.fold(0.0), 0, "a zero-input pass steps nowhere");
    assert_eq!(scrub.fold(-100.0), -2, "negative points step down");
}
