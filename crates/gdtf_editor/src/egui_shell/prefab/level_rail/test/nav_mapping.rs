use gdtf_battle_sim::metric::Level;

use super::support::size;
use crate::{
    canvas::CurrentEditLevel,
    egui_shell::prefab::level_rail::{rail_ui::rail_rows_top_first, scrub::ScrubAccumulator},
};

#[test]
fn click_jump_routes_through_the_kept_clamp() {
    let size = size();
    let jumped = CurrentEditLevel::jumped(Level::new(1), size);
    assert_eq!(*jumped.level(), 1, "an in-range click lands on its storey");
    let clamped = CurrentEditLevel::jumped(Level::new(7), size);
    assert_eq!(
        *clamped.level(),
        2,
        "an over-extent target clamps to levels-1 (the kept clamp)"
    );
}

#[test]
fn rows_are_top_storey_first() {
    let rows: Vec<u8> = rail_rows_top_first(3).map(|level| *level).collect();
    assert_eq!(rows, vec![2, 1, 0], "top storey first, ground last");
}

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
