//! The frame delta every harness app in this suite pins, and the check that it holds.

use core::time::Duration;

use bevy::{app::App, time::Time};

/// The delta a harness app reads every frame, so no wall clock reaches `Time`.
pub(crate) const PINNED_DELTA: Duration = Duration::ZERO;

/// Run two frames and assert the app read the pinned delta on the second.
///
/// Bevy's first update has no previous instant, so the second frame is the one to read.
pub(crate) fn assert_reads_a_pinned_delta(app: &mut App) {
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<Time>().delta(),
        PINNED_DELTA,
        "a harness app must read the pinned delta, not whatever the machine took",
    );
}
