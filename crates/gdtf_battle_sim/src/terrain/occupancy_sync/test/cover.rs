//! C5 — the `CoverDestroyed` message sync into the grid's destroyed-cover set.

use super::{super::CoverDestroyed, support::*};

/// C9(c) — emitting a [`CoverDestroyed`] message and ticking once adds the cell
/// to the grid's destroyed-cover set.
///
/// Writes the message into the world buffer, ticks, and asserts the cell now
/// reads destroyed (and an unrelated cell does not). In place — no rebuild.
#[test]
fn cover_destroyed_message_marks_the_cell() {
    let mut app = headless_app();
    let smashed = key(30, 31, 3);
    let intact = key(0, 0, 0);

    app.world_mut().write_message(CoverDestroyed::new(smashed));
    app.update();

    assert_eq!(
        cover_destroyed(&app, smashed),
        Some(true),
        "a CoverDestroyed message must mark its cell destroyed (C5)",
    );
    assert_eq!(
        cover_destroyed(&app, intact),
        Some(false),
        "an unrelated cell must not be marked destroyed",
    );
}
