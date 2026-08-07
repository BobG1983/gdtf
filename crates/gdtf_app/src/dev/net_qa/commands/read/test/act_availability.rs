//! Where the classic acts answer: a running battle whose screen has caught up.

use gdtf_qa_protocol::command::{CommandAvailability, RefusalNote, UnavailableCode};

use crate::dev::net_qa::{
    commands::set::GAME_COMMANDS,
    facts::{BattleModel, GameFacts, PlaybackCatchUp, StepperActivity, TurnOwner},
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

/// A battle whose turn belongs to the player, which is where a classic act is taken.
fn in_a_battle(battlescape: BattleScapePhaseNet, catch_up: PlaybackCatchUp) -> GameFacts {
    battle_facts(battlescape, catch_up, TurnOwner::Player)
}

fn battle_facts(
    battlescape: BattleScapePhaseNet,
    catch_up: PlaybackCatchUp,
    turn_owner: TurnOwner,
) -> GameFacts {
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
        turn_owner,
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

/// The note a refused command answers with, so two refusals can be compared.
fn refusal_note(name: &str, facts: GameFacts) -> RefusalNote {
    let CommandAvailability::Unavailable { note, .. } = availability(name, facts) else {
        unreachable!("`{name}` must refuse these facts");
    };
    note
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
    for name in CLASSIC_ACTS.iter().filter(|name| **name != "act.end_turn") {
        assert_eq!(
            refusal_note("act.end_turn", at_the_menu()),
            refusal_note(name, at_the_menu()),
            "`act.end_turn` must refuse the menu for the same reason `{name}` does: no battle is \
             running, which is not a turn-owner refusal",
        );
    }
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
fn act_end_turn_refuses_when_the_turn_is_not_the_players() {
    let facts = battle_facts(
        BattleScapePhaseNet::BattleRunning,
        PlaybackCatchUp::CaughtUp,
        TurnOwner::OtherFaction,
    );

    let CommandAvailability::Unavailable { code, note } = availability("act.end_turn", facts)
    else {
        unreachable!("`act.end_turn` must refuse while another faction is acting");
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "ending another faction's turn is a state the host is not in, not a replay wait",
    );
    assert!(
        !note.as_str().is_empty(),
        "`act.end_turn`'s refusal must say the turn belongs to another faction",
    );

    for name in CLASSIC_ACTS.iter().filter(|name| **name != "act.end_turn") {
        assert_eq!(
            availability(name, facts),
            CommandAvailability::Available,
            "`{name}` is taken on whoever is selected, so an enemy turn must not refuse it",
        );
    }
}

#[test]
fn act_end_turn_refuses_wrong_state_when_the_screen_is_behind_on_another_factions_turn() {
    let facts = battle_facts(
        BattleScapePhaseNet::BattleRunning,
        PlaybackCatchUp::Behind,
        TurnOwner::OtherFaction,
    );

    let CommandAvailability::Unavailable { code, note } = availability("act.end_turn", facts)
    else {
        unreachable!("`act.end_turn` must refuse while another faction is acting");
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "an enemy turn outlasts the replay, so a client told to wait for the screen would retry \
         into the same refusal",
    );
    assert!(
        !note.as_str().is_empty(),
        "`act.end_turn`'s refusal must say the turn belongs to another faction",
    );

    for name in CLASSIC_ACTS.iter().filter(|name| **name != "act.end_turn") {
        let CommandAvailability::Unavailable { code, .. } = availability(name, facts) else {
            unreachable!("`{name}` must refuse while the screen is behind the log");
        };
        assert_eq!(
            code,
            UnavailableCode::Replaying,
            "`{name}` reads no turn owner, so the replay is still the only reason it cannot act",
        );
    }
}

#[test]
fn every_classic_act_refuses_replaying_while_the_screen_is_behind_the_log() {
    assert_refused(
        "while the screen is still playing the log back",
        in_a_battle(BattleScapePhaseNet::BattleRunning, PlaybackCatchUp::Behind),
        UnavailableCode::Replaying,
    );
}
