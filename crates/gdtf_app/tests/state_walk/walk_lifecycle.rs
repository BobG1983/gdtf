use bevy::{
    app::{App, AppExit},
    state::state::State,
};
use gdtf_app::test_support::{
    AfterMathState, AppState, BattleRunningComplete, BattleScapeState, RunningState, app_state,
};
use gdtf_test_utils::advance_until;

use super::harness::*;

fn drive_to_teardown() -> App {
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

    for _ in 0..WALK_BUDGET {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236); the \
             placeholder turn-budget auto-exit must not advance it",
        );
        assert_ne!(
            app_state(&app),
            AppState::Teardown,
            "the walk must NOT reach Teardown on its own — it rests at BattleRunning until an \
             explicit end signal",
        );
    }

    app.world_mut().insert_resource(BattleRunningComplete);
    let reached = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached,
        "an explicit BattleRunningComplete insert should advance the walk to AppState::Teardown \
         within {WALK_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );

    app
}

#[test]
fn full_walk_reaches_teardown() {
    let app = drive_to_teardown();
    assert_eq!(
        app_state(&app),
        AppState::Teardown,
        "the driven walk should rest in AppState::Teardown",
    );
}

#[test]
fn teardown_emits_app_exit() {
    let mut app = drive_to_teardown();

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
        "Teardown's move_on must emit AppExit::Success within {WALK_BUDGET} updates of reaching \
         Teardown (GTW-311); observed {observed_exit:?}",
    );
}

#[test]
fn deep_pop_to_quit() {
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

    let reached_quit = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Quit),
        WALK_BUDGET,
    );
    assert!(
        reached_quit,
        "the AfterMath terminal should pop all the way out to RunningState::Quit, not wrap back \
         into the game; last observed RunningState was {:?}",
        running_state(&app),
    );

    let reached_teardown = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached_teardown,
        "RunningState::Quit should advance AppState to Teardown; last observed AppState was {:?}",
        app_state(&app),
    );
}
