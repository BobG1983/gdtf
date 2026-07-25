//! Exhaustive per-variant round-trip + parity-forcing pins for [`NetIntent`] and its
//! payload enums (GTW-734).

use crate::{
    ids::{
        CellLevelNet, CellNet, CellXNet, CellYNet, DoorToken, EmplacementToken, FireModeIndex,
        FocusTargetNet, GangerToken, LevelNet, PointerPosNet, PointerXNet, PointerYNet,
    },
    intent::{
        AimNet, FacingNet, KeyNet, KeyPressNet, KeybindActionNet, MeleeTargetNet, NetIntent,
        StanceNet, UiStackNet,
    },
    test_support::assert_ron_round_trip,
};

/// One sample cell for building intent cases.
fn a_cell() -> CellNet {
    CellNet::new(CellXNet::new(3), CellYNet::new(4))
}

/// One sample 3D `(cell, storey)` key for the intents that aim/step/lob in three
/// dimensions ([`NetIntent::Fire`] / [`NetIntent::Move`] / [`NetIntent::ThrowGrenade`]).
/// The non-zero storey keeps the round-trip honest — a level-dropping regression fails.
fn a_cell_level() -> CellLevelNet {
    CellLevelNet::new(a_cell(), LevelNet::new(2))
}

/// Every [`NetIntent`] variant — the round-trip table. Kept in lock-step with the enum
/// by [`net_intent_is_exhaustive`]: adding a variant breaks that witness's `match`
/// until it (and this list) gain the new arm — the parity force.
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
        NetIntent::SwapUiStack {
            stack: UiStackNet::Egui,
        },
    ]
}

/// The wildcard-free witness: this `match` fails to compile the moment a variant is
/// added to [`NetIntent`], forcing the new variant into [`net_intent_cases`] (and thus
/// the round-trip) before the suite can go green again.
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
        | NetIntent::SetFocus { .. }
        | NetIntent::SwapUiStack { .. } => {}
    }
}

/// Every [`NetIntent`] variant round-trips through compact RON identically.
#[test]
fn net_intent_round_trips_every_variant() {
    let cases = net_intent_cases();
    assert_eq!(
        cases.len(),
        25,
        "the case table lists every NetIntent variant"
    );
    for case in &cases {
        net_intent_is_exhaustive(case);
        assert_ron_round_trip(case);
    }
}

/// Every [`StanceNet`] variant round-trips; the witness forces new variants in.
#[test]
fn stance_net_round_trips_every_variant() {
    for stance in [StanceNet::Standing, StanceNet::Crouching, StanceNet::Prone] {
        match stance {
            StanceNet::Standing | StanceNet::Crouching | StanceNet::Prone => {}
        }
        assert_ron_round_trip(&stance);
    }
}

/// Every [`FacingNet`] compass variant round-trips; the witness forces new variants in.
#[test]
fn facing_net_round_trips_every_variant() {
    for facing in [
        FacingNet::North,
        FacingNet::NorthEast,
        FacingNet::East,
        FacingNet::SouthEast,
        FacingNet::South,
        FacingNet::SouthWest,
        FacingNet::West,
        FacingNet::NorthWest,
    ] {
        match facing {
            FacingNet::North
            | FacingNet::NorthEast
            | FacingNet::East
            | FacingNet::SouthEast
            | FacingNet::South
            | FacingNet::SouthWest
            | FacingNet::West
            | FacingNet::NorthWest => {}
        }
        assert_ron_round_trip(&facing);
    }
}

/// Both [`MeleeTargetNet`] variants + the [`AimNet`] flag round-trip.
#[test]
fn melee_target_and_aim_round_trip() {
    for target in [
        MeleeTargetNet::Ganger(GangerToken::new(1)),
        MeleeTargetNet::Structure(CellLevelNet::new(a_cell(), LevelNet::new(2))),
    ] {
        match target {
            MeleeTargetNet::Ganger(_) | MeleeTargetNet::Structure(_) => {}
        }
        assert_ron_round_trip(&target);
    }
    assert_ron_round_trip(&AimNet::new(true));
    assert_ron_round_trip(&AimNet::new(false));
}

/// Every [`KeyNet`] physical key round-trips; the witness forces new variants in.
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
        KeyNet::ArrowUp,
        KeyNet::ArrowDown,
        KeyNet::ArrowLeft,
        KeyNet::ArrowRight,
        KeyNet::F9,
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
            | KeyNet::ArrowUp
            | KeyNet::ArrowDown
            | KeyNet::ArrowLeft
            | KeyNet::ArrowRight
            | KeyNet::F9 => {}
        }
        assert_ron_round_trip(&key);
    }
}

/// Every [`KeybindActionNet`] bound action round-trips; the witness forces new variants in.
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

/// Both [`KeyPressNet`] forms + a [`FocusTargetNet`] round-trip.
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

/// Every [`UiStackNet`] variant round-trips; the witness forces new variants in — a new
/// UI stack under comparison must gain its wire name here before the suite goes green.
#[test]
fn ui_stack_net_round_trips_every_variant() {
    for stack in [UiStackNet::BevyUi, UiStackNet::Egui] {
        match stack {
            UiStackNet::BevyUi | UiStackNet::Egui => {}
        }
        assert_ron_round_trip(&stack);
    }
}
