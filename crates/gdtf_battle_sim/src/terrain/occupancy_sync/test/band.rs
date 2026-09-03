use super::support::*;
use crate::{
    clearance::silhouette_band,
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
    occupancy::BodyOcclusion,
};

#[test]
fn occupant_band_tracks_stance() {
    let mut app = headless_app();
    let at = key(11, 4, 0);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Crouching),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(silhouette_band(StanceKind::Crouching)),
        "a kneeling ganger publishes the MID silhouette band",
    );
    assert_eq!(grid_band(&app, at), Some(HeightBand::Mid));

    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();
    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the occupant stays put on an in-place re-pose",
    );
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::Low),
        "re-posing prone re-publishes the LOW band at the unchanged slot ",
    );
}

#[test]
fn dead_ganger_clears_its_band() {
    let mut app = headless_app();
    let at = key(14, 15, 1);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(at),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();
    app.update();
    assert_eq!(
        grid_band(&app, at),
        Some(HeightBand::High),
        "the alive ganger's band is published",
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
    assert_eq!(
        grid_band(&app, at),
        None,
        "a dead ganger's band must be cleared too ",
    );
    assert_eq!(
        grid_body(&app, at),
        Some(BodyOcclusion::new(ganger, HeightBand::Low)),
        "the corpse must be recorded in the body channel on the floor; found {:?}",
        grid_body(&app, at),
    );

    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Crouching);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        None,
        "a stance written on a corpse must not put it back in the occupant slot; found {:?}",
        grid_occupant(&app, at),
    );
    assert_eq!(
        grid_body(&app, at),
        Some(BodyOcclusion::new(ganger, HeightBand::Low)),
        "the corpse stays in the body channel after the stance write; found {:?}",
        grid_body(&app, at),
    );
}
