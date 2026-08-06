use crate::dev::net_qa::{
    facts::{
        BattleModel, BattleScreen, GameFacts, PlaybackCatchUp, PresenterReadiness, StepperActivity,
    },
    wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
};

fn facts(
    game: Option<GamePhaseNet>,
    battlescape: Option<BattleScapePhaseNet>,
    model: BattleModel,
) -> GameFacts {
    GameFacts::new(
        AppPhaseNet::new(
            LifecyclePhaseNet::Running,
            Some(RunningPhaseNet::Game),
            game,
            battlescape,
            None,
        ),
        model,
        StepperActivity::NotStepping,
        PlaybackCatchUp::CaughtUp,
    )
}

#[test]
fn the_battle_screen_stays_open_through_every_battlescape_phase() {
    for phase in [
        BattleScapePhaseNet::Generation,
        BattleScapePhaseNet::AnimateIn,
        BattleScapePhaseNet::BattleRunning,
        BattleScapePhaseNet::AnimateOut,
        BattleScapePhaseNet::AfterMath,
    ] {
        assert_eq!(
            facts(
                Some(GamePhaseNet::BattleScape),
                Some(phase),
                BattleModel::Absent
            )
            .battle_screen(),
            BattleScreen::Open,
            "a roster or log read stays answerable during `{phase:?}`",
        );
    }

    for layer in [GamePhaseNet::Setup, GamePhaseNet::HiveScape] {
        assert_eq!(
            facts(Some(layer), None, BattleModel::Present).battle_screen(),
            BattleScreen::Closed,
            "`{layer:?}` is not the battle screen even with a battle loaded",
        );
    }

    assert_eq!(
        facts(None, None, BattleModel::Absent).battle_screen(),
        BattleScreen::Closed,
        "no game layer at all is not the battle screen",
    );
}

#[test]
fn the_presenter_is_ready_only_while_running_with_a_loaded_battle() {
    assert_eq!(
        facts(
            Some(GamePhaseNet::BattleScape),
            Some(BattleScapePhaseNet::BattleRunning),
            BattleModel::Present,
        )
        .presenter_readiness(),
        PresenterReadiness::Ready,
        "a running battle with its sim state loaded is what the panels read",
    );

    assert_eq!(
        facts(
            Some(GamePhaseNet::BattleScape),
            Some(BattleScapePhaseNet::BattleRunning),
            BattleModel::Absent,
        )
        .presenter_readiness(),
        PresenterReadiness::NotReady,
        "without the sim's battle resource the offer and inspect systems never run",
    );

    assert_eq!(
        facts(
            Some(GamePhaseNet::BattleScape),
            Some(BattleScapePhaseNet::AnimateIn),
            BattleModel::Present,
        )
        .presenter_readiness(),
        PresenterReadiness::NotReady,
        "a loaded battle that is not running yet has no panels up",
    );
}
