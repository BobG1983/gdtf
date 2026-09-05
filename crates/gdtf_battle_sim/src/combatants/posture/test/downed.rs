use crate::{
    ganger::{LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    test_support::SimAppBuilder,
};

#[test]
fn a_ganger_that_goes_down_or_dies_is_written_prone() {
    let mut app = SimAppBuilder::new().with_acts().build();

    let standing = |x: i32| {
        (
            Position::new(CellLevel::new(Cell::new(x, 10), Level::new(0))),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        )
    };
    let (downed, dead) = {
        let world = app.world_mut();
        (
            world.spawn(standing(10)).id(),
            world.spawn(standing(12)).id(),
        )
    };
    app.update();

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(downed) {
        *life = LifeState::Downed;
    }
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(dead) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        app.world().get::<Stance>(downed).copied(),
        Some(Stance::new(StanceKind::Prone)),
        "a ganger that goes down lies prone — found {:?}",
        app.world().get::<Stance>(downed).copied(),
    );
    assert_eq!(
        app.world().get::<Stance>(dead).copied(),
        Some(Stance::new(StanceKind::Prone)),
        "a ganger that dies lies prone — found {:?}",
        app.world().get::<Stance>(dead).copied(),
    );
}
