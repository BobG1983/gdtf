use gdtf_game::test_support::{BattleRunningComplete, BattleScapeState};
use gdtf_test_utils::advance_until;

use super::harness::*;

const PERSIST_UPDATES: u32 = 32;

#[test]
fn battle_running_persists_without_an_end_signal() {
    let mut app = driven_battle_app();

    for iteration in 0..PERSIST_UPDATES {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted — it must not auto-advance to AnimateOut; failed on update {iteration} of {PERSIST_UPDATES}",
        );
    }
}

#[test]
fn explicit_end_marker_advances_to_animate_out() {
    let mut app = driven_battle_app();

    app.world_mut().insert_resource(BattleRunningComplete);

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateOut)
    });
}

#[test]
fn battle_running_persists_with_outcome_system_wired_but_quiescent() {
    let mut app = driven_battle_app();

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
