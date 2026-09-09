//! Where the view and battle controls answer: a running battle with its sim state loaded.

use cobalt_mcp_protocol::command::{CommandAvailability, UnavailableCode};

use crate::dev::mcp::{
    commands::set::GAME_COMMANDS,
    facts::{BattleModel, GameFacts, PlaybackCatchUp, StepperActivity, TurnOwner},
    wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
};

/// The seven controls, all of which share one availability word.
const CONTROLS: &[&str] = &[
    "view.level_up",
    "view.level_down",
    "view.toggle_full_view",
    "view.toggle_reachable_overlay",
    "view.pan",
    "view.look_at",
    "battle.set_fire_mode",
];

fn in_a_battle(
    battlescape: BattleScapePhaseNet,
    model: BattleModel,
    catch_up: PlaybackCatchUp,
) -> GameFacts {
    GameFacts::new(
        AppPhaseNet::new(
            LifecyclePhaseNet::Running,
            Some(RunningPhaseNet::Game),
            Some(GamePhaseNet::BattleScape),
            Some(battlescape),
            None,
        ),
        model,
        StepperActivity::NotStepping,
        catch_up,
        TurnOwner::Player,
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
        TurnOwner::OtherFaction,
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

fn assert_answering(what: &str, facts: GameFacts) {
    for name in CONTROLS {
        assert_eq!(
            availability(name, facts),
            CommandAvailability::Available,
            "`{name}` drives the view or the panels {what}, so its word must say so",
        );
    }
}

fn assert_refused(what: &str, facts: GameFacts) {
    for name in CONTROLS {
        let CommandAvailability::Unavailable { code, note } = availability(name, facts) else {
            unreachable!("`{name}` must refuse {what}");
        };
        assert_eq!(
            code,
            UnavailableCode::WrongState,
            "a control refuses a wrong host state with WrongState: `{name}` {what} — {note:?}",
        );
        assert!(
            !note.as_str().is_empty(),
            "`{name}`'s refusal must name the precondition that is missing",
        );
    }
}

#[test]
fn every_control_answers_in_a_running_battle_with_its_sim_state_loaded() {
    assert_answering(
        "in a running battle with its sim state loaded",
        in_a_battle(
            BattleScapePhaseNet::BattleRunning,
            BattleModel::Present,
            PlaybackCatchUp::CaughtUp,
        ),
    );
}

#[test]
fn every_control_still_answers_while_the_screen_is_playing_the_log_back() {
    assert_answering(
        "while the screen is still playing the act log back",
        in_a_battle(
            BattleScapePhaseNet::BattleRunning,
            BattleModel::Present,
            PlaybackCatchUp::Behind,
        ),
    );
}

#[test]
fn every_control_refuses_wrong_state_outside_a_running_loaded_battle() {
    assert_refused("at the menu", at_the_menu());
    assert_refused(
        "while the battle is still generating",
        in_a_battle(
            BattleScapePhaseNet::Generation,
            BattleModel::Absent,
            PlaybackCatchUp::CaughtUp,
        ),
    );
    assert_refused(
        "on the aftermath screen",
        in_a_battle(
            BattleScapePhaseNet::AfterMath,
            BattleModel::Present,
            PlaybackCatchUp::CaughtUp,
        ),
    );
    assert_refused(
        "in a running battle whose sim state is gone",
        in_a_battle(
            BattleScapePhaseNet::BattleRunning,
            BattleModel::Absent,
            PlaybackCatchUp::CaughtUp,
        ),
    );
}
