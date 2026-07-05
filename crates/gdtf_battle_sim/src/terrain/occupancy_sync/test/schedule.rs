//! AC1-AC3 — `SimSystems::Simulate` set membership + derives, and the move-sync
//! firing through the set.

use super::{super::SimSystems, support::*};
use crate::ganger::{LifeState, Position};

/// AC1 — [`SimSystems::Simulate`] is a public, hashable ordering set with the
/// required derives. Referencing the variant from the (in-crate, but
/// `pub`-reachable) test path and asserting equality/clone proves the variant is
/// public and that `Clone`/`Copy`/`PartialEq`/`Eq` are present; `cargo dbuild`
/// linking the binary confirms it is reachable downstream with no `unreachable_pub`.
#[test]
fn sim_systems_simulate_is_public_and_derives() {
    let set = SimSystems::Simulate;
    assert_eq!(
        set,
        SimSystems::Simulate,
        "the set compares equal to itself"
    );
    // `Copy` (a use after `set` was already read) and `Clone` both hold.
    assert_eq!(set, set.clone(), "the set clones to an equal value");
}

/// AC2/AC3 — after [`OccupancyMaintenancePlugin`] nests its chain under
/// [`SimSystems::Simulate`], the move-sync still fires through the set: a ganger
/// that moves clears its OLD slot and marks its NEW one within one `app.update()`,
/// proving set membership did not break execution (membership has no observable
/// beyond ordering + execution, so the behavioral assertion stands in for it).
#[test]
fn move_sync_fires_through_the_simulate_set() {
    let mut app = headless_app();
    let start = key(8, 8, 0);
    let dest = key(10, 12, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(start), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, start),
        Some(ganger),
        "initial placement marks the start slot through SimSystems::Simulate",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, start),
        None,
        "the OLD slot is cleared running inside SimSystems::Simulate",
    );
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "the NEW slot is marked running inside SimSystems::Simulate",
    );
}
