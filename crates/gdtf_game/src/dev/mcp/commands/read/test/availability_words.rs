//! Which availability word each battle read carries, pinned command by command.

use cobalt_mcp_protocol::command::{CommandAvailability, UnavailableCode};

use crate::dev::mcp::{
    commands::set::GAME_COMMANDS,
    facts::{BattleModel, GameFacts, PlaybackCatchUp, StepperActivity, TurnOwner},
    wire::{AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
};

/// The battle reads, each of which must appear in a case below.
const BATTLE_READS: &[&str] = &[
    "battle.roster",
    "battle.turn",
    "battle.selection",
    "battle.offers",
    "battle.inspect",
    "battle.sightline",
    "battle.visible",
    "battle.reachable",
    "battle.cost",
    "log.read",
    "log.omniscient_read",
];

/// Commands these battle-read cases say nothing about.
const NOT_BATTLE_READS: &[&str] = &[
    "app.phase",
    "capture.screenshot",
    "settings.read",
    "ui.focus",
    "playback.state",
    "battle.start",
    "battle.flee",
    "procgen.step",
    "wait",
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
    "act.melee",
    "act.shove",
    "act.stabilize",
    "act.execute",
    "act.throw_grenade",
    "act.open_door",
    "act.enter_emplacement",
    "act.exit_emplacement",
    "input.press_key",
    "input.hover",
    "input.set_focus",
    "input.focus_step",
    "input.activate",
    "input.click_cell",
    "view.level_up",
    "view.level_down",
    "view.toggle_full_view",
    "view.pan",
    "view.look_at",
    "battle.set_fire_mode",
];

fn on_the_battle_screen(battlescape: BattleScapePhaseNet, model: BattleModel) -> GameFacts {
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
        PlaybackCatchUp::CaughtUp,
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

/// Ask the registered command set, which is the same list the router admits calls from.
fn availability(name: &str, facts: GameFacts) -> CommandAvailability {
    let Some(command) = GAME_COMMANDS
        .iter()
        .find(|command| command.name().as_str() == name)
    else {
        unreachable!("`{name}` must be registered in GAME_COMMANDS to have an availability word");
    };
    command.availability(&facts)
}

/// Require exactly `answering` of the battle reads to be available on this host, and no others.
fn assert_answering(what: &str, facts: GameFacts, answering: &[&str]) {
    for name in BATTLE_READS {
        let expected_to_answer = answering.contains(name);
        match availability(name, facts) {
            CommandAvailability::Available => assert!(
                expected_to_answer,
                "`{name}` answers {what}, but its availability word must refuse there",
            ),
            CommandAvailability::Unavailable { code, note } => {
                assert!(
                    !expected_to_answer,
                    "`{name}` must answer {what}; it refused with {code:?} — {note:?}",
                );
                assert_eq!(
                    code,
                    UnavailableCode::WrongState,
                    "a battle read refuses a wrong host state with WrongState: `{name}` \
                     {what} — {note:?}",
                );
            }
        }
    }
}

#[test]
fn every_battle_read_refuses_at_the_menu() {
    assert_answering("at the menu", at_the_menu(), &[]);
}

#[test]
fn the_battle_screen_reads_answer_before_the_battle_is_running() {
    assert_answering(
        "while the battle is still generating",
        on_the_battle_screen(BattleScapePhaseNet::Generation, BattleModel::Absent),
        &[
            "battle.roster",
            "battle.turn",
            "battle.selection",
            "log.read",
            "log.omniscient_read",
        ],
    );
}

#[test]
fn the_battle_screen_reads_answer_after_the_battle_is_over() {
    assert_answering(
        "on the aftermath screen",
        on_the_battle_screen(BattleScapePhaseNet::AfterMath, BattleModel::Present),
        &[
            "battle.roster",
            "battle.turn",
            "battle.selection",
            "log.read",
            "log.omniscient_read",
        ],
    );
}

#[test]
fn the_sightline_and_cost_reads_answer_on_a_running_battle_the_panels_have_not_loaded() {
    assert_answering(
        "in a running battle whose sim state is gone",
        on_the_battle_screen(BattleScapePhaseNet::BattleRunning, BattleModel::Absent),
        &[
            "battle.roster",
            "battle.turn",
            "battle.selection",
            "log.read",
            "log.omniscient_read",
            "battle.sightline",
            "battle.cost",
        ],
    );
}

#[test]
fn every_battle_read_answers_in_a_running_loaded_battle() {
    assert_answering(
        "in a running battle with its sim state loaded",
        on_the_battle_screen(BattleScapePhaseNet::BattleRunning, BattleModel::Present),
        BATTLE_READS,
    );
}

#[test]
fn every_registered_command_is_classified_by_these_cases() {
    for command in GAME_COMMANDS {
        let name = command.name();
        let name = name.as_str();
        assert!(
            BATTLE_READS.contains(&name) || NOT_BATTLE_READS.contains(&name),
            "`{name}` is registered but no case here says where it answers; add it to \
             BATTLE_READS with its cases, or to NOT_BATTLE_READS",
        );
    }
}
