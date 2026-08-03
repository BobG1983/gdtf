use super::assert_ron_round_trip;
use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a phase value serializes to compact RON");
    };
    text
}

const LIFECYCLE: [AppState; 5] = [
    AppState::Init,
    AppState::Load,
    AppState::Intro,
    AppState::Running,
    AppState::Teardown,
];

const RUNNING: [RunningState; 4] = [
    RunningState::Menu,
    RunningState::Game,
    RunningState::Options,
    RunningState::Quit,
];

const GAME: [GameState; 3] = [
    GameState::Setup,
    GameState::HiveScape,
    GameState::BattleScape,
];

const BATTLESCAPE: [BattleScapeState; 5] = [
    BattleScapeState::Generation,
    BattleScapeState::AnimateIn,
    BattleScapeState::BattleRunning,
    BattleScapeState::AnimateOut,
    BattleScapeState::AfterMath,
];

const AFTERMATH: [AfterMathState; 3] = [
    AfterMathState::AnimateIn,
    AfterMathState::DisplayAftermath,
    AfterMathState::AnimateOut,
];

fn assert_mirrors_by_name<S: core::fmt::Debug, M: core::fmt::Debug>(
    states: &[S],
    mirror: impl Fn(&S) -> M,
    family: &str,
) {
    let mut seen: Vec<String> = Vec::with_capacity(states.len());
    for state in states {
        let mirrored = format!("{:?}", mirror(state));
        assert_eq!(
            mirrored,
            format!("{state:?}"),
            "{family}: the wire mirror of {state:?} must carry that state's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{family}: {state:?} maps onto {mirrored}, which another state already claims — \
             two states that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_lifecycle_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(&LIFECYCLE, LifecyclePhaseNet::from_state, "AppState");
}

#[test]
fn every_running_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &RUNNING,
        |state| RunningPhaseNet::from_state(*state),
        "RunningState",
    );
}

#[test]
fn every_game_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(&GAME, |state| GamePhaseNet::from_state(*state), "GameState");
}

#[test]
fn every_battlescape_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &BATTLESCAPE,
        |state| BattleScapePhaseNet::from_state(*state),
        "BattleScapeState",
    );
}

#[test]
fn every_aftermath_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &AFTERMATH,
        |state| AfterMathPhaseNet::from_state(*state),
        "AfterMathState",
    );
}

#[test]
fn every_state_mirror_arm_round_trips() {
    for state in &LIFECYCLE {
        assert_ron_round_trip(&LifecyclePhaseNet::from_state(state));
    }
    for state in RUNNING {
        assert_ron_round_trip(&RunningPhaseNet::from_state(state));
    }
    for state in GAME {
        assert_ron_round_trip(&GamePhaseNet::from_state(state));
    }
    for state in BATTLESCAPE {
        assert_ron_round_trip(&BattleScapePhaseNet::from_state(state));
    }
    for state in AFTERMATH {
        assert_ron_round_trip(&AfterMathPhaseNet::from_state(state));
    }
}

#[test]
fn the_five_level_phase_round_trips() {
    assert_ron_round_trip(&AppPhaseNet::new(
        LifecyclePhaseNet::Running,
        Some(RunningPhaseNet::Game),
        Some(GamePhaseNet::BattleScape),
        Some(BattleScapePhaseNet::AfterMath),
        Some(AfterMathPhaseNet::DisplayAftermath),
    ));
    assert_ron_round_trip(&AppPhaseNet::new(
        LifecyclePhaseNet::Init,
        None,
        None,
        None,
        None,
    ));
}

#[test]
fn phase_values_serialize_under_their_own_names() {
    assert_eq!(encoded(&LifecyclePhaseNet::Running), "Running");
    assert_eq!(encoded(&RunningPhaseNet::Game), "Game");
    assert_eq!(encoded(&GamePhaseNet::BattleScape), "BattleScape");
    assert_eq!(encoded(&BattleScapePhaseNet::AfterMath), "AfterMath");
    assert_eq!(
        encoded(&AfterMathPhaseNet::DisplayAftermath),
        "DisplayAftermath"
    );
    assert_eq!(
        encoded(&AppPhaseNet::new(
            LifecyclePhaseNet::Init,
            None,
            None,
            None,
            None
        )),
        "(app:Init,running:None,game:None,battlescape:None,aftermath:None)",
    );
}

#[test]
fn an_absent_level_serializes_as_an_explicit_null() {
    let phase = AppPhaseNet::new(
        LifecyclePhaseNet::from_state(&AppState::Running),
        Some(RunningPhaseNet::from_state(RunningState::Menu)),
        None,
        None,
        None,
    );
    let Ok(encoded) = serde_json::to_value(phase) else {
        unreachable!("the phase record serializes");
    };
    assert_eq!(encoded["app"], "Running");
    assert_eq!(encoded["running"], "Menu");
    for level in ["game", "battlescape", "aftermath"] {
        assert_eq!(
            encoded.get(level),
            Some(&serde_json::Value::Null),
            "the inactive level `{level}` must be present and null: {encoded}",
        );
    }
}
