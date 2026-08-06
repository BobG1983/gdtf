//! Where the classic acts answer: a running battle whose screen has caught up.

use gdtf_qa_protocol::command::{CommandAvailability, UnavailableCode};

use crate::dev::net_qa::{
    commands::set::GAME_COMMANDS,
    facts::{BattleModel, GameFacts, PlaybackCatchUp, StepperActivity},
    wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
};

/// The classic acts, all of which share one availability word.
const CLASSIC_ACTS: &[&str] = &[
    "act.select",
    "act.select_next",
    "act.select_prev",
    "act.select_clear",
    "act.move",
    "act.fire",
    "act.reload",
    "act.set_stance",
    "act.set_aiming",
    "act.set_facing",
    "act.end_turn",
];

fn in_a_battle(battlescape: BattleScapePhaseNet, catch_up: PlaybackCatchUp) -> GameFacts {
    GameFacts::new(
        AppPhaseNet::new(
            LifecyclePhaseNet::Running,
            Some(RunningPhaseNet::Game),
            Some(GamePhaseNet::BattleScape),
            Some(battlescape),
            None,
        ),
        BattleModel::Present,
        StepperActivity::NotStepping,
        catch_up,
    )
}

fn at_the_menu() -> GameFacts {
    GameFacts::new(
        AppPhaseNet::new(
            LifecyclePhaseNet::Running,
            Some(RunningPhaseNet::Menu),
            None,
            None,
            None,
        ),
        BattleModel::Absent,
        StepperActivity::NotStepping,
        PlaybackCatchUp::CaughtUp,
    )
}

fn availability(name: &str, facts: GameFacts) -> CommandAvailability {
    let Some(command) = GAME_COMMANDS
        .iter()
        .find(|command| command.name().as_str() == name)
    else {
        unreachable!("`{name}` must be registered in GAME_COMMANDS to have an availability word");
    };
    command.availability(&facts)
}

fn assert_refused(what: &str, facts: GameFacts, code: UnavailableCode) {
    for name in CLASSIC_ACTS {
        let CommandAvailability::Unavailable { code: live, note } = availability(name, facts)
        else {
            unreachable!("`{name}` must refuse {what}");
        };
        assert_eq!(live, code, "`{name}` must name why it cannot act {what}");
        assert!(
            !note.as_str().is_empty(),
            "`{name}`'s refusal must name the precondition that is missing",
        );
    }
}

#[test]
fn every_classic_act_answers_in_a_running_battle_the_screen_has_caught_up_with() {
    for name in CLASSIC_ACTS {
        assert_eq!(
            availability(
                name,
                in_a_battle(
                    BattleScapePhaseNet::BattleRunning,
                    PlaybackCatchUp::CaughtUp
                )
            ),
            CommandAvailability::Available,
            "`{name}` acts in a caught-up running battle, so its word must say so",
        );
    }
}

#[test]
fn every_classic_act_refuses_wrong_state_outside_a_running_battle() {
    assert_refused("at the menu", at_the_menu(), UnavailableCode::WrongState);
    assert_refused(
        "while the battle is still generating",
        in_a_battle(BattleScapePhaseNet::Generation, PlaybackCatchUp::CaughtUp),
        UnavailableCode::WrongState,
    );
    assert_refused(
        "on the aftermath screen",
        in_a_battle(BattleScapePhaseNet::AfterMath, PlaybackCatchUp::CaughtUp),
        UnavailableCode::WrongState,
    );
}

#[test]
fn every_classic_act_refuses_replaying_while_the_screen_is_behind_the_log() {
    assert_refused(
        "while the screen is still playing the log back",
        in_a_battle(BattleScapePhaseNet::BattleRunning, PlaybackCatchUp::Behind),
        UnavailableCode::Replaying,
    );
}
