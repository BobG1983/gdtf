use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    phase::{BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet},
    wait::{ActCountNet, AppPhaseTargetNet, WaitConditionNet},
};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a wait value serializes to compact RON");
    };
    text
}

fn deep_target() -> AppPhaseTargetNet {
    AppPhaseTargetNet::new(
        Some(LifecyclePhaseNet::Running),
        Some(RunningPhaseNet::Game),
        Some(GamePhaseNet::BattleScape),
        Some(BattleScapePhaseNet::BattleRunning),
        None,
    )
}

#[test]
fn every_wait_condition_round_trips() {
    for condition in [
        WaitConditionNet::CaughtUp,
        WaitConditionNet::Phase(deep_target()),
        WaitConditionNet::Phase(AppPhaseTargetNet::default()),
        WaitConditionNet::LogAtLeast(ActCountNet::new(3)),
        WaitConditionNet::WalkComplete,
        WaitConditionNet::TurnChanged,
        WaitConditionNet::BattleDecided,
        WaitConditionNet::GenerationComplete,
    ] {
        match condition {
            WaitConditionNet::CaughtUp
            | WaitConditionNet::Phase(_)
            | WaitConditionNet::LogAtLeast(_)
            | WaitConditionNet::WalkComplete
            | WaitConditionNet::TurnChanged
            | WaitConditionNet::BattleDecided
            | WaitConditionNet::GenerationComplete => {}
        }
        assert_ron_round_trip(&condition);
    }
    assert_ron_round_trip(&ActCountNet::new(0));
    assert_ron_round_trip(&AppPhaseTargetNet::default());
    assert_ron_round_trip(&deep_target());
}

#[test]
fn wait_values_serialize_under_their_own_names() {
    assert_eq!(encoded(&WaitConditionNet::CaughtUp), "CaughtUp");
    assert_eq!(encoded(&WaitConditionNet::WalkComplete), "WalkComplete");
    assert_eq!(encoded(&ActCountNet::new(7)), "7");
    assert_eq!(
        encoded(&WaitConditionNet::LogAtLeast(ActCountNet::new(7))),
        "LogAtLeast(7)",
    );
}

#[test]
fn an_unnamed_level_reads_back_as_a_wildcard() {
    let target = deep_target();
    assert_eq!(target.app(), Some(LifecyclePhaseNet::Running));
    assert_eq!(target.running(), Some(RunningPhaseNet::Game));
    assert_eq!(target.game(), Some(GamePhaseNet::BattleScape));
    assert_eq!(
        target.battlescape(),
        Some(BattleScapePhaseNet::BattleRunning)
    );
    assert_eq!(
        target.aftermath(),
        None,
        "a level the caller never named stays a wildcard",
    );
}

#[test]
fn a_target_naming_no_level_decodes_from_an_empty_record() {
    let Ok(decoded) = ron::de::from_str::<AppPhaseTargetNet>("()") else {
        unreachable!("every level defaults, so an empty record is a legal all-wildcard target");
    };
    assert_eq!(decoded, AppPhaseTargetNet::default());
}

#[test]
fn a_target_naming_a_level_that_does_not_exist_is_refused() {
    let hostile = "(hive:Some(Sump))";
    assert!(
        ron::de::from_str::<AppPhaseTargetNet>(hostile).is_err(),
        "`{hostile}` names a level the target does not declare and must not decode",
    );
}

#[test]
fn a_condition_this_host_does_not_offer_is_refused() {
    for hostile in ["Settled", "LogAtMost(3)"] {
        assert!(
            ron::de::from_str::<WaitConditionNet>(hostile).is_err(),
            "`{hostile}` is not a condition this host published and must not decode",
        );
    }
}
