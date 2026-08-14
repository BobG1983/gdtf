use bevy::{
    app::{App, AppExit},
    ecs::entity::Entity,
    state::state::State,
    ui::Interaction,
};
use gdtf_app::test_support::{
    AfterMathState, AppState, BattleRunningComplete, BattleScapeState, QuitButton, RunningState,
    app_state,
};
use gdtf_test_utils::advance_until;

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

fn drive_to_menu(app: &mut App) -> bool {
    advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        WALK_BUDGET,
    )
}

#[test]
fn a_finished_battle_returns_to_the_menu() {
    let mut app = walk_app_with_theme();

    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let reached_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        WALK_BUDGET,
    );
    assert!(
        reached_battle_running,
        "the walk should descend to BattleScapeState::BattleRunning within {WALK_BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app.world_mut().insert_resource(BattleRunningComplete);

    let reached_aftermath_out = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<BattleScapeState>>()
                .is_some_and(|state| *state.get() == BattleScapeState::AfterMath)
                && app
                    .world()
                    .get_resource::<State<AfterMathState>>()
                    .is_some_and(|state| *state.get() == AfterMathState::AnimateOut)
        },
        WALK_BUDGET,
    );
    assert!(
        reached_aftermath_out,
        "the walk should descend into BattleScapeState::AfterMath / AfterMathState::AnimateOut \
         within {WALK_BUDGET} updates",
    );

    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        WALK_BUDGET,
    );
    assert!(
        reached_menu,
        "the AfterMath terminal should return to RunningState::Menu so another battle can be \
         started; last observed RunningState was {:?}",
        running_state(&app),
    );
    assert_eq!(
        app.should_exit(),
        None,
        "a finished battle must leave the process running, not ask the app to exit",
    );
}

#[test]
fn the_menu_quit_action_emits_app_exit() {
    let mut app = walk_app_with_theme();

    assert!(
        drive_to_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let found = single_with::<QuitButton>(&mut app);
    assert!(
        found.is_some(),
        "the menu must spawn exactly one QuitButton for this case to press",
    );
    let quit = found.unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(quit) {
        *interaction = Interaction::Pressed;
    }

    let reached_teardown = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached_teardown,
        "pressing Quit should carry RunningState::Quit through to AppState::Teardown; last \
         observed AppState was {:?}",
        app_state(&app),
    );

    let mut observed_exit = None;
    for _ in 0..WALK_BUDGET {
        app.update();
        if let Some(exit) = app.should_exit() {
            observed_exit = Some(exit);
            break;
        }
    }

    assert_eq!(
        observed_exit,
        Some(AppExit::Success),
        "Teardown's move_on must emit AppExit::Success within {WALK_BUDGET} updates of reaching Teardown; observed {observed_exit:?}",
    );
}
