//! Every arm of the five state mirrors, and the nesting `AppPhaseNet` carries (GTW-942).
//!
//! The mirrors are the one place a state can be renamed on the wire without the compiler
//! noticing: `from_state` is wildcard-free, so a MISSING arm fails to build, but a SWAPPED
//! arm (`RunningState::Menu => Self::Options`) builds and ships a lie. These cases pin each
//! arm by name — the mirror's variant name must equal the state's — and then pin that the
//! two are one-to-one, so no future arm can quietly fold two states onto one mirror.

use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

/// Every [`AppState`], in declaration order.
const LIFECYCLE: [AppState; 5] = [
    AppState::Init,
    AppState::Load,
    AppState::Intro,
    AppState::Running,
    AppState::Teardown,
];

/// Every [`RunningState`], in declaration order.
const RUNNING: [RunningState; 4] = [
    RunningState::Menu,
    RunningState::Game,
    RunningState::Options,
    RunningState::Quit,
];

/// Every [`GameState`], in declaration order.
const GAME: [GameState; 3] = [
    GameState::Setup,
    GameState::HiveScape,
    GameState::BattleScape,
];

/// Every [`BattleScapeState`], in declaration order.
const BATTLESCAPE: [BattleScapeState; 5] = [
    BattleScapeState::Generation,
    BattleScapeState::AnimateIn,
    BattleScapeState::BattleRunning,
    BattleScapeState::AnimateOut,
    BattleScapeState::AfterMath,
];

/// Every [`AfterMathState`], in declaration order.
const AFTERMATH: [AfterMathState; 3] = [
    AfterMathState::AnimateIn,
    AfterMathState::DisplayAftermath,
    AfterMathState::AnimateOut,
];

/// Assert each state maps to the mirror variant of the SAME name, and that the mapping is
/// one-to-one.
///
/// Comparing the two `Debug` names is what catches a swapped arm: the mirror is a rename of
/// the state, so any arm whose two sides disagree is the defect, whatever the names are.
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

/// Every [`AppState`] mirrors to its own [`LifecyclePhaseNet`] name.
#[test]
fn every_lifecycle_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(&LIFECYCLE, LifecyclePhaseNet::from_state, "AppState");
}

/// Every [`RunningState`] mirrors to its own [`RunningPhaseNet`] name.
#[test]
fn every_running_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &RUNNING,
        |state| RunningPhaseNet::from_state(*state),
        "RunningState",
    );
}

/// Every [`GameState`] mirrors to its own [`GamePhaseNet`] name.
#[test]
fn every_game_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(&GAME, |state| GamePhaseNet::from_state(*state), "GameState");
}

/// Every [`BattleScapeState`] mirrors to its own [`BattleScapePhaseNet`] name.
#[test]
fn every_battlescape_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &BATTLESCAPE,
        |state| BattleScapePhaseNet::from_state(*state),
        "BattleScapeState",
    );
}

/// Every [`AfterMathState`] mirrors to its own [`AfterMathPhaseNet`] name.
#[test]
fn every_aftermath_state_mirrors_to_its_own_name() {
    assert_mirrors_by_name(
        &AFTERMATH,
        |state| AfterMathPhaseNet::from_state(*state),
        "AfterMathState",
    );
}

/// A phase serializes with all five levels present, the absent ones as explicit `null`.
///
/// "The app is not in a running game" and "I could not read the running screen" must not look
/// the same to a client, so a level that is not live is a `null` KEY rather than a missing one.
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
