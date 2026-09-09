use bevy::{
    app::{App, AppExit},
    ecs::entity::Entity,
    state::state::State,
    ui::Interaction,
};
use cobalt_test_utils::advance_until;
use gdtf_game::test_support::{
    AfterMathState, AppState, BattleRunningComplete, BattleScapeState, QuitButton, RunningState,
    app_state,
};

use super::harness::*;

fn single_with<M: bevy::ecs::component::Component>(app: &mut App) -> Option<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<M>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn drive_to_menu(app: &mut App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
}

#[test]
fn a_finished_battle_returns_to_the_menu() {
    let mut app = walk_app_with_theme();

    drive_past_menu(&mut app);

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app.world_mut().insert_resource(BattleRunningComplete);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<BattleScapeState>>()
            .is_some_and(|state| *state.get() == BattleScapeState::AfterMath)
            && app
                .world()
                .get_resource::<State<AfterMathState>>()
                .is_some_and(|state| *state.get() == AfterMathState::AnimateOut)
    });

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    assert_eq!(
        app.should_exit(),
        None,
        "a finished battle must leave the process running, not ask the app to exit",
    );
}

#[test]
fn the_menu_quit_action_emits_app_exit() {
    let mut app = walk_app_with_theme();

    drive_to_menu(&mut app);

    let found = single_with::<QuitButton>(&mut app);
    assert!(
        found.is_some(),
        "the menu must spawn exactly one QuitButton for this case to press",
    );
    let quit = found.unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(quit) {
        *interaction = Interaction::Pressed;
    }

    advance_until(&mut app, |app| app_state(app) == AppState::Teardown);

    let observed_exit = loop {
        app.update();
        if let Some(exit) = app.should_exit() {
            break exit;
        }
    };

    assert_eq!(
        observed_exit,
        AppExit::Success,
        "Teardown's move_on must emit AppExit::Success; observed {observed_exit:?}",
    );
}
