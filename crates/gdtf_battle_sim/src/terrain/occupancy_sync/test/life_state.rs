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
fn downed_ganger_retains_its_slot() {
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
        "a downed ganger HOLDS its occupant slot — only Dead frees it ",
    );
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::High),
        "a downed ganger retains its silhouette band, still occluding fire ",
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
