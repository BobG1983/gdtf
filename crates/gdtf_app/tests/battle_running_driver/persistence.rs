use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

const PERSIST_UPDATES: u32 = 32;

#[test]
fn battle_running_persists_without_an_end_signal() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236) — it must \
             not auto-advance to AnimateOut; failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}

#[test]
fn explicit_end_marker_advances_to_animate_out() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    app.world_mut().insert_resource(BattleRunningComplete);

    let reached_animate_out = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateOut),
        BUDGET,
    );
    assert!(
        reached_animate_out,
        "an explicit BattleRunningComplete insert must trip move_on and advance BattleRunning → \
         AnimateOut within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

#[test]
fn battle_running_persists_with_outcome_system_wired_but_quiescent() {
    let app_opt = driven_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert!(
            app.world()
                .get_resource::<BattleRunningComplete>()
                .is_none(),
            "with NO outcome signal, end_battle_on_outcome must insert NO BattleRunningComplete \
             marker; failed on update {iteration} of {PERSIST_UPDATES}",
        );
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with end_battle_on_outcome wired-but-quiescent (no \
             outcome); failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}
