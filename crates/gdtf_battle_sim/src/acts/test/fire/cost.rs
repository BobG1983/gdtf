//! The fire charge and the arc turn charge both come from shared cost functions.

use super::support::*;
use crate::posture::turn_tu_cost;

#[test]
fn a_volley_charges_exactly_mode_tu_cost() {
    let mut app = headless_app();
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
    let _target = place_arc_target(&mut app, 8, 5);
    let quoted = fire_cost_in(&app);
    assert!(quoted.is_some(), "the app must carry combat tuning");
    let before = app.world().get::<Tu>(shooter).map(|tu| **tu);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    assert_eq!(
        before
            .zip(app.world().get::<Tu>(shooter).map(|tu| **tu))
            .map(|(b, a)| b - a),
        quoted,
        "the volley must charge exactly what mode_tu_cost quoted for the same mode",
    );
}

#[test]
fn an_out_of_arc_shot_charges_the_shared_turn_quote_plus_the_fire_quote() {
    let mut app = headless_app();
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
    let _target = place_arc_target(&mut app, 5, 8);
    let turn_leaf = app
        .world()
        .get_resource::<CombatTuning>()
        .map(|tuning| tuning.turn_tu);
    let steps = Direction::East.steps_to(Direction::South);
    let quoted = turn_leaf
        .map(|leaf| turn_tu_cost(steps, &leaf))
        .zip(fire_cost_in(&app))
        .map(|(turn, fire)| *turn + fire);
    assert!(quoted.is_some(), "the app must carry combat tuning");
    let before = app.world().get::<Tu>(shooter).map(|tu| **tu);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 8),
        Level::new(0),
    ));
    app.update();

    assert_eq!(
        app.world().get::<Facing>(shooter).map(|facing| **facing),
        Some(Direction::South),
        "an out-of-arc shot turns to face the target",
    );
    assert_eq!(
        before
            .zip(app.world().get::<Tu>(shooter).map(|tu| **tu))
            .map(|(b, a)| b - a),
        quoted,
        "the arc turn must charge the shared turn_tu_cost, not a second copy of the math",
    );
}
