use super::{super::SimSystems, support::*};
use crate::ganger::{LifeState, Position};

#[test]
fn sim_systems_simulate_is_public_and_derives() {
    let set = SimSystems::Simulate;
    assert_eq!(
        set,
        SimSystems::Simulate,
        "the set compares equal to itself"
    );
    assert_eq!(set, set.clone(), "the set clones to an equal value");
}

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
