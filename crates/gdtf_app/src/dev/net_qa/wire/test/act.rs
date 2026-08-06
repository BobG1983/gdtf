use crate::dev::net_qa::wire::{
    act::NetIntent,
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    key::{KeyNet, KeyPressNet, KeybindActionNet},
    misc::FireModeIndex,
    pointer::{PointerPosNet, PointerXNet, PointerYNet},
    test::assert_ron_round_trip,
    token::{DoorToken, EmplacementToken, FocusTargetNet, GangerToken},
};

fn a_cell() -> CellNet {
    CellNet::new(CellXNet::new(3), CellYNet::new(4))
}

fn a_cell_level() -> CellLevelNet {
    CellLevelNet::new(a_cell(), LevelNet::new(2))
}

fn net_intent_cases() -> Vec<NetIntent> {
    vec![
        NetIntent::Fire {
            target: a_cell_level(),
            mode:   FireModeIndex::new(1),
        },
        NetIntent::Move {
            dest: a_cell_level(),
        },
        NetIntent::SetStance {
            stance: StanceNet::Prone,
        },
        NetIntent::SetAiming {
            aim: AimNet::new(true),
        },
        NetIntent::SetFacing {
            facing: FacingNet::SouthWest,
        },
        NetIntent::Reload,
        NetIntent::EndTurn,
        NetIntent::Melee {
            target: MeleeTargetNet::Ganger(GangerToken::new(11)),
        },
        NetIntent::Shove {
            target: GangerToken::new(12),
        },
        NetIntent::Stabilize {
            target: GangerToken::new(13),
        },
        NetIntent::Execute {
            target: GangerToken::new(14),
        },
        NetIntent::ThrowGrenade {
            target: a_cell_level(),
        },
        NetIntent::OpenDoor {
            target: DoorToken::new(15),
        },
        NetIntent::EnterEmplacement {
            target: EmplacementToken::new(16),
        },
        NetIntent::ExitEmplacement {
            target: EmplacementToken::new(17),
        },
        NetIntent::Select {
            target: GangerToken::new(18),
        },
        NetIntent::SelectNext,
        NetIntent::SelectPrev,
        NetIntent::SelectionClear,
        NetIntent::LevelUp,
        NetIntent::LevelDown,
        NetIntent::PressKey {
            key: KeyPressNet::Key(KeyNet::Tab),
        },
        NetIntent::Hover {
            at: PointerPosNet::new(PointerXNet::new(120), PointerYNet::new(48)),
        },
        NetIntent::SetFocus {
            target: FocusTargetNet::new(19),
        },
    ]
}

fn net_intent_is_exhaustive(intent: &NetIntent) {
    match intent {
        NetIntent::Fire { .. }
        | NetIntent::Move { .. }
        | NetIntent::SetStance { .. }
        | NetIntent::SetAiming { .. }
        | NetIntent::SetFacing { .. }
        | NetIntent::Reload
        | NetIntent::EndTurn
        | NetIntent::Melee { .. }
        | NetIntent::Shove { .. }
        | NetIntent::Stabilize { .. }
        | NetIntent::Execute { .. }
        | NetIntent::ThrowGrenade { .. }
        | NetIntent::OpenDoor { .. }
        | NetIntent::EnterEmplacement { .. }
        | NetIntent::ExitEmplacement { .. }
        | NetIntent::Select { .. }
        | NetIntent::SelectNext
        | NetIntent::SelectPrev
        | NetIntent::SelectionClear
        | NetIntent::LevelUp
        | NetIntent::LevelDown
        | NetIntent::PressKey { .. }
        | NetIntent::Hover { .. }
        | NetIntent::SetFocus { .. } => {}
    }
}

#[test]
fn net_intent_round_trips_every_variant() {
    let cases: Vec<NetIntent> = net_intent_cases();
    assert_eq!(
        cases.len(),
        24,
        "the case table lists every NetIntent variant"
    );
    for case in &cases {
        net_intent_is_exhaustive(case);
        assert_ron_round_trip(case);
    }
}

#[test]
fn key_net_round_trips_every_variant() {
    for key in [
        KeyNet::Escape,
        KeyNet::KeyQ,
        KeyNet::KeyE,
        KeyNet::KeyC,
        KeyNet::KeyF,
        KeyNet::KeyR,
        KeyNet::KeyV,
        KeyNet::Tab,
        KeyNet::PageUp,
        KeyNet::PageDown,
        KeyNet::BracketLeft,
        KeyNet::BracketRight,
        KeyNet::Digit1,
        KeyNet::Digit2,
        KeyNet::Digit3,
        KeyNet::Digit4,
        KeyNet::Digit5,
        KeyNet::Digit6,
        KeyNet::Digit7,
        KeyNet::Digit8,
        KeyNet::Digit9,
        KeyNet::Enter,
        KeyNet::ArrowUp,
        KeyNet::ArrowDown,
        KeyNet::ArrowLeft,
        KeyNet::ArrowRight,
    ] {
        match key {
            KeyNet::Escape
            | KeyNet::KeyQ
            | KeyNet::KeyE
            | KeyNet::KeyC
            | KeyNet::KeyF
            | KeyNet::KeyR
            | KeyNet::KeyV
            | KeyNet::Tab
            | KeyNet::PageUp
            | KeyNet::PageDown
            | KeyNet::BracketLeft
            | KeyNet::BracketRight
            | KeyNet::Digit1
            | KeyNet::Digit2
            | KeyNet::Digit3
            | KeyNet::Digit4
            | KeyNet::Digit5
            | KeyNet::Digit6
            | KeyNet::Digit7
            | KeyNet::Digit8
            | KeyNet::Digit9
            | KeyNet::Enter
            | KeyNet::ArrowUp
            | KeyNet::ArrowDown
            | KeyNet::ArrowLeft
            | KeyNet::ArrowRight => {}
        }
        assert_ron_round_trip(&key);
        assert_eq!(
            KeyNet::from_bound(key.bound()),
            key,
            "the wire key and the keybind vocabulary must name the same key both ways",
        );
    }
}

#[test]
fn keybind_action_net_round_trips_every_variant() {
    for action in [
        KeybindActionNet::SelectClear,
        KeybindActionNet::LevelUp,
        KeybindActionNet::LevelDown,
        KeybindActionNet::ToggleFullView,
        KeybindActionNet::StanceCycle,
        KeybindActionNet::AimToggle,
        KeybindActionNet::FacingCycle,
        KeybindActionNet::SelectNext,
        KeybindActionNet::SelectPrev,
    ] {
        match action {
            KeybindActionNet::SelectClear
            | KeybindActionNet::LevelUp
            | KeybindActionNet::LevelDown
            | KeybindActionNet::ToggleFullView
            | KeybindActionNet::StanceCycle
            | KeybindActionNet::AimToggle
            | KeybindActionNet::FacingCycle
            | KeybindActionNet::SelectNext
            | KeybindActionNet::SelectPrev => {}
        }
        assert_ron_round_trip(&action);
    }
}

#[test]
fn key_press_and_focus_target_round_trip() {
    for press in [
        KeyPressNet::Key(KeyNet::Escape),
        KeyPressNet::Action(KeybindActionNet::SelectClear),
    ] {
        match press {
            KeyPressNet::Key(_) | KeyPressNet::Action(_) => {}
        }
        assert_ron_round_trip(&press);
    }
    assert_ron_round_trip(&FocusTargetNet::new(7));
}
