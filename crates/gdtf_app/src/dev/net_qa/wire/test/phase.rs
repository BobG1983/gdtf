//! Every arm of the five state mirrors, and the nesting `AppPhaseNet` carries (GTW-942,
//! round-trip cases added by GTW-944).
//!
//! The mirrors are the one place a state can be renamed on the wire without the compiler
//! noticing: `from_state` is wildcard-free, so a MISSING arm fails to build, but a SWAPPED
//! arm (`RunningState::Menu => Self::Options`) builds and ships a lie. These cases pin each
//! arm by name — the mirror's variant name must equal the state's — and then pin that the
//! two are one-to-one, so no future arm can quietly fold two states onto one mirror.
//!
//! The name cases never touch serde, so they are only half the story: a `serde(skip)` on a
//! variant, or a broken `Deserialize`, ships an `app.phase` that cannot encode a real phase
//! while every name case still passes. The round-trip and wire-text cases below are the
//! other half, and they are what clause 5 of GTW-944 owes for these six types.

use super::assert_ron_round_trip;
use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

/// The compact RON `value` encodes to.
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom) if it cannot encode.
fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a phase value serializes to compact RON");
    };
    text
}

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

/// EVERY arm of all five mirrors survives a compact-RON round trip — value → text → value.
///
/// One case per arm rather than one representative: `serde` attributes are per-variant, so a
/// representative proves nothing about its neighbours. The arm tables above are the same
/// ones the name cases walk, so an arm added to a state is round-tripped the moment its
/// `from_state` arm exists.
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

/// The composed [`AppPhaseNet`] round-trips in both shapes `app.phase` publishes — every
/// level live, and the four nested levels absent.
///
/// The all-live case is what catches a lost nested level: a dropped `Option` field decodes
/// as `None` under a laxer struct and the value would come back CHANGED.
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

/// Each phase rides the wire under its OWN variant name, and the composed record as the
/// five named fields — neither of which a round trip alone can see, because a
/// `serde(rename)` round-trips perfectly while changing what a client reads.
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
