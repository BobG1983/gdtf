use crate::dev::mcp::{
    facts::{BattleActivity, BattleModel, GameFacts, PlaybackCatchUp, StepperActivity, TurnOwner},
    wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
};

fn facts_with(battlescape: Option<BattleScapePhaseNet>) -> GameFacts {
    GameFacts::new(
        AppPhaseNet::new(
            LifecyclePhaseNet::Running,
            Some(RunningPhaseNet::Game),
            Some(GamePhaseNet::BattleScape),
            battlescape,
            None,
        ),
        BattleModel::Absent,
        StepperActivity::NotStepping,
        PlaybackCatchUp::CaughtUp,
        TurnOwner::Player,
    )
}

#[test]
fn only_the_battle_running_phase_counts_as_running() {
    assert_eq!(
        facts_with(Some(BattleScapePhaseNet::BattleRunning)).battle_activity(),
        BattleActivity::Running,
        "a live battle is the one phase a command may act in",
    );

    for phase in [
        BattleScapePhaseNet::Generation,
        BattleScapePhaseNet::AnimateIn,
        BattleScapePhaseNet::AnimateOut,
        BattleScapePhaseNet::AfterMath,
    ] {
        assert_eq!(
            facts_with(Some(phase)).battle_activity(),
            BattleActivity::NotRunning,
            "`{phase:?}` is not a running battle",
        );
    }

    assert_eq!(
        facts_with(None).battle_activity(),
        BattleActivity::NotRunning,
        "no battlescape state at all is not a running battle",
    );
}
