use bevy::time::Time;

use super::{PINNED_DELTA, WindowedTestAppBuilder};

#[test]
fn the_harness_app_reads_a_pinned_delta() {
    let mut app = WindowedTestAppBuilder::new().build();

    // Bevy's first update has no previous instant, so the second frame is the one to read.
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<Time>().delta(),
        PINNED_DELTA,
        "a windowed harness app must read the pinned delta, not whatever the machine took",
    );
}
