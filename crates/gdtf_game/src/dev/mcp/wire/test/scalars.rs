//! lost `#[serde(transparent)]`: RON round-trips a non-transparent newtype perfectly well,
use super::assert_ron_round_trip;
use crate::dev::mcp::wire::{
    misc::{FireModeIndex, FrameDelay, ModeKindNet, RequestId, SeedNet, SituationRef, ViewModeNet},
    pointer::{MouseButtonNet, PointerPosNet, PointerXNet, PointerYNet},
    shell::{CaughtUpNet, SoundNet},
    token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
};

fn encoded<T: serde::Serialize>(value: &T) -> String {
    let Ok(text) = ron::ser::to_string(value) else {
        unreachable!("a wire scalar serializes to compact RON");
    };
    text
}

#[test]
fn entity_tokens_round_trip() {
    assert_ron_round_trip(&GangerToken::new(42));
    assert_ron_round_trip(&DoorToken::new(7));
    assert_ron_round_trip(&EmplacementToken::new(9));
    assert_ron_round_trip(&FocusTargetNet::new(11));
}

#[test]
fn scalar_handles_round_trip() {
    assert_ron_round_trip(&FireModeIndex::new(2));
    assert_ron_round_trip(&SituationRef::new("skirmish".to_owned()));
    assert_ron_round_trip(&SeedNet::new(0xDEAD_BEEF));
    assert_ron_round_trip(&FrameDelay::new(15));
    assert_ron_round_trip(&RequestId::new(1));
}

#[test]
fn every_fire_mode_kind_round_trips() {
    for kind in [ModeKindNet::Single, ModeKindNet::Burst, ModeKindNet::Full] {
        match kind {
            ModeKindNet::Single | ModeKindNet::Burst | ModeKindNet::Full => {}
        }
        assert_ron_round_trip(&kind);
    }
    assert_eq!(encoded(&ModeKindNet::Burst), "Burst");
}

#[test]
fn every_view_mode_round_trips() {
    for mode in [ViewModeNet::DownToActive, ViewModeNet::FullView] {
        match mode {
            ViewModeNet::DownToActive | ViewModeNet::FullView => {}
        }
        assert_ron_round_trip(&mode);
    }
    assert_eq!(encoded(&ViewModeNet::FullView), "FullView");
}

#[test]
fn shell_read_flags_round_trip() {
    assert_ron_round_trip(&SoundNet::new(true));
    assert_ron_round_trip(&SoundNet::new(false));
    assert_ron_round_trip(&CaughtUpNet::new(true));
    assert_ron_round_trip(&CaughtUpNet::new(false));
}

#[test]
fn pointer_positions_round_trip() {
    assert_ron_round_trip(&PointerXNet::new(-3));
    assert_ron_round_trip(&PointerYNet::new(58));
    assert_ron_round_trip(&PointerPosNet::new(
        PointerXNet::new(-3),
        PointerYNet::new(58),
    ));
}

#[test]
fn mouse_buttons_round_trip() {
    for button in [MouseButtonNet::Left, MouseButtonNet::Right] {
        match button {
            MouseButtonNet::Left | MouseButtonNet::Right => {}
        }
        assert_ron_round_trip(&button);
    }
    assert_eq!(encoded(&MouseButtonNet::Left), "Left");
    assert_eq!(encoded(&MouseButtonNet::Right), "Right");
}

/// Every `#[serde(transparent)]` scalar rides the wire as its BARE inner value, not a
#[test]
fn every_scalar_serializes_transparently() {
    assert_eq!(encoded(&GangerToken::new(42)), "42");
    assert_eq!(encoded(&DoorToken::new(7)), "7");
    assert_eq!(encoded(&EmplacementToken::new(9)), "9");
    assert_eq!(encoded(&FocusTargetNet::new(11)), "11");
    assert_eq!(encoded(&FireModeIndex::new(2)), "2");
    assert_eq!(
        encoded(&SituationRef::new("skirmish".to_owned())),
        "\"skirmish\""
    );
    assert_eq!(encoded(&SeedNet::new(255)), "255");
    assert_eq!(encoded(&FrameDelay::new(15)), "15");
    assert_eq!(encoded(&RequestId::new(1)), "1");
    assert_eq!(encoded(&PointerXNet::new(-3)), "-3");
    assert_eq!(encoded(&PointerYNet::new(58)), "58");
    assert_eq!(encoded(&SoundNet::new(true)), "true");
    assert_eq!(encoded(&CaughtUpNet::new(false)), "false");
}

#[test]
fn a_pointer_position_serializes_as_a_named_pair() {
    assert_eq!(
        encoded(&PointerPosNet::new(
            PointerXNet::new(1),
            PointerYNet::new(2)
        )),
        "(x:1,y:2)",
    );
}

#[test]
fn a_pointer_position_refuses_an_unknown_field() {
    let hostile = "(x:1,y:2,z:3)";
    assert!(
        ron::de::from_str::<PointerPosNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
