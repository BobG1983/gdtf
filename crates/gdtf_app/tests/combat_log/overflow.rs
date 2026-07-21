//! FIFO overflow: the oldest lines despawn past the tuned cap.

use bevy::prelude::*;
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_sim::{acts::MovementOccurred, prelude::Cell};

use super::harness::*;

/// The shipped default visible-line cap (`combat_log.tuning.ron` / `MaxVisibleLines::DEFAULT`) — the
/// FIFO trim target this test asserts against. Mirrors the tuning default so the overflow test
/// is independent of the (unloaded, defaulted) RON in the headless harness.
const DEFAULT_MAX_VISIBLE: usize = 6;

/// The same cap as an `i32` for cell-coordinate arithmetic (the movement events' `y` is an `i32`
/// cell coord), so the overflow test needs no `usize`→`i32` cast (clippy `cast_possible_wrap`).
const DEFAULT_MAX_VISIBLE_I32: i32 = 6;

// ---------------------------------------------------------------------------------
// FIFO overflow — over the tuned cap, the OLDEST lines despawn; the newest survive.
// ---------------------------------------------------------------------------------

/// Appending MORE lines than the tuned `max_visible_lines` (default 6) FIFO-despawns the OLDEST,
/// so the visible count is capped and the most-recent lines are the survivors.
#[test]
fn overflow_fifo_despawns_the_oldest_lines() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // Write more movement events than the cap. Each `to` cell's y is the event index (an i32 cell
    // coord), so the rendered text is uniquely identifiable per line — the survivors are the
    // highest indices.
    let total: i32 = DEFAULT_MAX_VISIBLE_I32 + 3;
    for y in 0..total {
        play(
            &mut app,
            MovementOccurred::new(ganger, Cell::new(0, 0), Cell::new(0, y)),
        );
    }
    // Two updates: the first drains all events + appends + trims; a second settles any deferred
    // despawns from the trim so the visible set is stable for the assert.
    app.update();
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        DEFAULT_MAX_VISIBLE,
        "the visible line count is capped at max_visible_lines ({DEFAULT_MAX_VISIBLE}), got {} \
         lines: {texts:?}",
        texts.len(),
    );
    // The OLDEST (lowest y) are gone; the newest (the last cap-many indices) survive.
    let survivors_start = total - DEFAULT_MAX_VISIBLE_I32;
    for i in survivors_start..total {
        let expected = format!("Vex moved (0, 0) -> (0, {i})");
        assert!(
            texts.contains(&expected),
            "the newest line {expected:?} must survive the FIFO trim, got {texts:?}",
        );
    }
    // The very first (oldest) line must be gone.
    let oldest = "Vex moved (0, 0) -> (0, 0)".to_owned();
    assert!(
        !texts.contains(&oldest),
        "the OLDEST line {oldest:?} must have been FIFO-despawned, got {texts:?}",
    );
}
