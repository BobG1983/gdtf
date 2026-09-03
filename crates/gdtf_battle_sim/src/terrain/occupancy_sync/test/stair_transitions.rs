use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{LifeState, Position, Stance, StanceKind},
};

#[test]
fn move_off_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(8, 8, 2);
    let upper = key(8, 8, 3);
    let dest = key(9, 9, 2);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must be written on the stair",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(dest);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "old lower (stair) slot must be cleared",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "old upper slot must be cleared when moving off the stair ",
    );
    assert_eq!(grid_band(&app, upper), None, "old upper band must be gone");
    assert_eq!(
        grid_occupant(&app, dest),
        Some(ganger),
        "new destination must be occupied",
    );
    assert_eq!(grid_occupant(&app, key(9, 9, 3)), None);
}

#[test]
fn go_prone_in_place_on_stair_clears_upper() {
    let mut app = headless_app();
    let stair = key(3, 3, 0);
    let upper = key(3, 3, 1);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence must exist before going prone",
    );

    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger),
        "lower slot stays occupied (still on the stair)",
    );
    assert_eq!(
        grid_band(&app, stair),
        Some(HeightBand::Low),
        "lower slot now carries Low band (Prone)",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must be cleared when going prone in place (Test 7)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

#[test]
fn stair_to_stair_move_relocates_upper() {
    let mut app = headless_app();
    let stair_a = key(5, 5, 1);
    let upper_a = key(5, 5, 2);
    let stair_b = key(15, 15, 1);
    let upper_b = key(15, 15, 2);

    mark_stair(&mut app, stair_a);
    mark_stair(&mut app, stair_b);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair_a),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(grid_occupant(&app, upper_a), Some(ganger));

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
        *pos = Position::new(stair_b);
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, upper_a),
        None,
        "old upper must be cleared on stair-to-stair move",
    );
    assert_eq!(grid_band(&app, upper_a), None);
    assert_eq!(
        grid_occupant(&app, upper_b),
        Some(ganger),
        "new upper must be written on the new stair ",
    );
    assert_eq!(grid_band(&app, upper_b), Some(HeightBand::Low));
}

#[test]
fn dead_on_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(7, 7, 3);
    let upper = key(7, 7, 4);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence before death",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Dead;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        None,
        "lower slot must be cleared when DEAD ",
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "upper slot must also be cleared when killed on a stair (Test 9)",
    );
    assert_eq!(grid_band(&app, upper), None);
}

#[test]
fn downed_on_stair_clears_upper_presence() {
    let mut app = headless_app();
    let stair = key(7, 7, 3);
    let upper = key(7, 7, 4);

    mark_stair(&mut app, stair);

    let ganger = app
        .world_mut()
        .spawn((
            Position::new(stair),
            Stance::new(StanceKind::Standing),
            LifeState::Alive,
        ))
        .id();

    app.update();
    assert_eq!(
        grid_occupant(&app, upper),
        Some(ganger),
        "upper presence before going down",
    );

    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    app.update();

    assert_eq!(
        grid_occupant(&app, stair),
        Some(ganger),
        "a downed stair-occupant HOLDS its lower cell; found {:?}",
        grid_occupant(&app, stair),
    );
    assert_eq!(
        grid_band(&app, stair),
        Some(HeightBand::Low),
        "a downed body on a stair lies on the floor of its lower cell; found {:?}",
        grid_band(&app, stair),
    );
    assert_eq!(
        grid_occupant(&app, upper),
        None,
        "a body claims no upper cell — it is on the floor; found {:?}",
        grid_occupant(&app, upper),
    );
    assert_eq!(
        grid_band(&app, upper),
        None,
        "the released upper cell carries no band; found {:?}",
        grid_band(&app, upper),
    );
}
