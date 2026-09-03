use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
};

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
