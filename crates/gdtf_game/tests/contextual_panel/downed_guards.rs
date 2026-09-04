//! One case per conjunct of `can_execute` and `can_stabilize`, driven through the panel.

use bevy::prelude::*;
use gdtf_battle_sim::{ganger::Tu, prelude::LifeState};
use gdtf_game::test_support::{ExecuteButton, StabilizeButton};

use super::{actors::*, harness::*};

/// An alive actor beside a downed enemy: the pair `can_execute` allows.
fn an_offered_execute() -> (App, Entity, Entity) {
    let mut app = battle_running_app();
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 6, 6, 1, None);
    app.update();
    assert!(
        execute_visible(&mut app),
        "sanity: this pair must offer Execute before one term of it is broken",
    );
    (app, actor, target)
}

/// An alive actor beside a downed, bleeding ally: the pair `can_stabilize` allows.
fn an_offered_stabilize() -> (App, Entity, Entity) {
    let mut app = battle_running_app();
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 5, 6, 0, Some(false));
    app.update();
    assert!(
        stabilize_visible(&mut app),
        "sanity: this pair must offer Stabilize before one term of it is broken",
    );
    (app, actor, target)
}

/// One more update, then whether the Execute button has gone.
fn execute_withdrawn(app: &mut App) -> bool {
    app.update();
    !execute_visible(app)
}

/// One more update, then whether the Stabilize button has gone.
fn stabilize_withdrawn(app: &mut App) -> bool {
    app.update();
    !stabilize_visible(app)
}

#[test]
fn a_downed_enemy_out_of_reach_withdraws_execute() {
    let (mut app, _actor, target) = an_offered_execute();
    move_to(&mut app, target, 20, 20);

    assert!(
        execute_withdrawn(&mut app),
        "Execute needs an 8-adjacent target, so moving the target away withdraws it",
    );
}

#[test]
fn a_downed_actor_is_offered_no_execute() {
    let (mut app, actor, _target) = an_offered_execute();
    set_life(&mut app, actor, LifeState::Downed);

    assert!(
        execute_withdrawn(&mut app),
        "Execute needs an alive actor, so downing the selected ganger withdraws it",
    );
}

#[test]
fn a_dead_actor_is_offered_no_execute() {
    let (mut app, actor, _target) = an_offered_execute();
    set_life(&mut app, actor, LifeState::Dead);

    assert!(
        execute_withdrawn(&mut app),
        "Execute needs an alive actor, so killing the selected ganger withdraws it",
    );
}

#[test]
fn an_alive_enemy_is_no_execute_target() {
    let (mut app, _actor, target) = an_offered_execute();
    set_life(&mut app, target, LifeState::Alive);

    assert!(
        execute_withdrawn(&mut app),
        "Execute needs a downed target, so standing the target up withdraws it",
    );
}

#[test]
fn a_dead_enemy_is_no_execute_target() {
    let (mut app, _actor, target) = an_offered_execute();
    set_life(&mut app, target, LifeState::Dead);

    assert!(
        execute_withdrawn(&mut app),
        "Execute needs a downed target, so a corpse withdraws it",
    );
}

#[test]
fn a_downed_ally_is_no_execute_target() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 0, None);
    app.update();

    assert!(
        !execute_visible(&mut app),
        "Execute needs an opposing target, so a downed ally offers nothing",
    );
}

#[test]
fn a_pool_below_the_execute_cost_greys_the_button_rather_than_hiding_it() {
    let (mut app, actor, _target) = an_offered_execute();
    let cost = execute_cost(&app);
    assert!(
        *cost > 0,
        "an execute must cost something or an unaffordable pool cannot exist",
    );

    set_pool(&mut app, actor, Tu::new(cost.saturating_sub(1)));
    app.update();
    assert!(
        execute_visible(&mut app),
        "an unaffordable execute stays on screen — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<ExecuteButton>(&mut app),
        "a pool below the execute cost greys the button out",
    );

    set_pool(&mut app, actor, cost);
    app.update();
    app.update();
    assert!(
        !button_greyed::<ExecuteButton>(&mut app),
        "a pool that covers the cost drops the disabled marker again",
    );
}

#[test]
fn a_downed_ally_out_of_reach_withdraws_stabilize() {
    let (mut app, _actor, target) = an_offered_stabilize();
    move_to(&mut app, target, 20, 20);

    assert!(
        stabilize_withdrawn(&mut app),
        "Stabilize needs an 8-adjacent target, so moving the target away withdraws it",
    );
}

#[test]
fn a_downed_actor_is_offered_no_stabilize() {
    let (mut app, actor, _target) = an_offered_stabilize();
    set_life(&mut app, actor, LifeState::Downed);

    assert!(
        stabilize_withdrawn(&mut app),
        "Stabilize needs an alive actor, so downing the selected ganger withdraws it",
    );
}

#[test]
fn a_dead_actor_is_offered_no_stabilize() {
    let (mut app, actor, _target) = an_offered_stabilize();
    set_life(&mut app, actor, LifeState::Dead);

    assert!(
        stabilize_withdrawn(&mut app),
        "Stabilize needs an alive actor, so killing the selected ganger withdraws it",
    );
}

#[test]
fn an_alive_ally_is_no_stabilize_target() {
    let (mut app, _actor, target) = an_offered_stabilize();
    set_life(&mut app, target, LifeState::Alive);

    assert!(
        stabilize_withdrawn(&mut app),
        "Stabilize needs a downed target, so standing the bleeding ally up withdraws it",
    );
}

#[test]
fn a_dead_ally_is_no_stabilize_target() {
    let (mut app, _actor, target) = an_offered_stabilize();
    set_life(&mut app, target, LifeState::Dead);

    assert!(
        stabilize_withdrawn(&mut app),
        "Stabilize needs a downed target, so a corpse withdraws it",
    );
}

#[test]
fn a_downed_enemy_is_no_stabilize_target() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    assert!(
        !stabilize_visible(&mut app),
        "Stabilize needs a friendly target, so a bleeding downed enemy offers nothing",
    );
}

#[test]
fn an_already_stabilized_downed_ally_is_no_stabilize_target() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 5, 6, 0, Some(true));
    app.update();

    assert!(
        !stabilize_visible(&mut app),
        "Stabilize needs a bleeding target, so an already stabilized ally offers nothing",
    );
}

#[test]
fn a_pool_below_the_stabilize_cost_greys_the_button_rather_than_hiding_it() {
    let (mut app, actor, _target) = an_offered_stabilize();
    let cost = stabilize_cost(&app);
    assert!(
        *cost > 0,
        "a stabilize must cost something or an unaffordable pool cannot exist",
    );

    set_pool(&mut app, actor, Tu::new(cost.saturating_sub(1)));
    app.update();
    assert!(
        stabilize_visible(&mut app),
        "an unaffordable stabilize stays on screen — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<StabilizeButton>(&mut app),
        "a pool below the stabilize cost greys the button out",
    );

    set_pool(&mut app, actor, cost);
    app.update();
    app.update();
    assert!(
        !button_greyed::<StabilizeButton>(&mut app),
        "a pool that covers the cost drops the disabled marker again",
    );
}
