use bevy::prelude::Messages;

use super::support::{PLAYER, bleed_rate, bleeding_ganger, end_turn, life_of, live_app, wounds_of};
use crate::{
    acts::{MoveRequested, MovementOccurred},
    ganger::LifeState,
    metric::{Cell, CellLevel, Level},
    test_support::{GangerEntityBuilder, full_vision},
};

fn drain_movements(app: &mut bevy::prelude::App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

#[test]
fn a_mid_turn_act_does_not_tick_the_bleed_clock() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    app.insert_resource(full_vision());

    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);
    let mover = GangerEntityBuilder::new()
        .at(CellLevel::new(Cell::new(10, 10), Level::new(0)))
        .faction(PLAYER)
        .life_state(LifeState::Alive)
        .tu(100)
        .spawn(app.world_mut());

    app.update();
    app.update();
    assert_eq!(
        wounds_of(&app, downed),
        start,
        "plain mid-turn frames must not tick the bleed clock (no turn boundary crossed)",
    );

    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    app.update();
    let movements = drain_movements(&mut app);
    assert_eq!(
        movements.len(),
        1,
        "the mid-turn move must actually RESOLVE (one MovementOccurred) — otherwise \
         this pin proves nothing: {movements:?}",
    );
    assert_eq!(
        wounds_of(&app, downed),
        start,
        "an act resolving mid-turn must NOT tick the bleed clock (GTW-641: bleeds \
         happen at turn start, never from act resolution)",
    );

    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "the bleed drains exactly once, at the turn-start boundary the End Turn crossed",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "the non-lethal boundary tick leaves the ganger Downed",
    );
}
