use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
    occupancy_sync::OccupancyMaintenancePlugin,
    test_support::SimAppBuilder,
};

#[test]
fn a_corpse_the_grid_released_stays_released() {
    let mut app = SimAppBuilder::new().with_acts().build();
    app.add_plugins(OccupancyMaintenancePlugin);
    let at = key(15, 15, 0);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        None,
        "the tick that kills a ganger releases its occupant slot; found {:?}",
        grid_occupant(&app, at),
    );

    // A stance write is the one thing that selects a corpse in `sync_moved_gangers` again.
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a stance write landing on the body after the death tick must not put the corpse back \
         in the slot; found {:?}",
        grid_occupant(&app, at),
    );
}

#[test]
fn dead_ganger_clears_its_slot() {
    let mut app = headless_app();
    let at = key(12, 13, 2);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the ganger must occupy its slot before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a dead ganger's occupant slot must be cleared (C4)",
    );
}

#[test]
fn downed_body_keeps_its_cell_and_loses_its_silhouette() {
    let mut app = headless_app();
    let at = key(20, 20, 0);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "a downed ganger HOLDS its occupant slot — only Dead frees it; found {:?}",
        grid_occupant(&app, at),
    );
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::Low),
        "a downed body lies on the floor, so it publishes the Low band; found {:?}",
        grid_band(&app, at),
    );

    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Crouching);
    }
    app.update();

    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::Low),
        "a stance written on a downed body must not raise it off the floor; found {:?}",
        grid_band(&app, at),
    );
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the downed body still holds its cell after the stance write; found {:?}",
        grid_occupant(&app, at),
    );
}

#[test]
fn still_alive_change_keeps_slot() {
    let mut app = headless_app();
    let at = key(7, 7, 1);

    let ganger = app
        .world_mut()
        .spawn((Position::new(at), LifeState::Alive))
        .id();
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Alive;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "an Alive LifeState change must NOT free the slot ",
    );
}
