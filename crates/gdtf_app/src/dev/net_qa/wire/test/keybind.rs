use gdtf_battle_input::{BoundKey, Keybinds};

use crate::dev::net_qa::wire::key::KeybindActionNet;

/// Nine distinct keys, so an action that reads the wrong row names a different key.
const TABLE: Keybinds = Keybinds {
    select_clear:     BoundKey::KeyQ,
    level_up:         BoundKey::KeyE,
    level_down:       BoundKey::KeyC,
    toggle_full_view: BoundKey::KeyF,
    stance_cycle:     BoundKey::KeyR,
    aim_toggle:       BoundKey::KeyV,
    facing_cycle:     BoundKey::KeyTab,
    select_next:      BoundKey::KeyPageUp,
    select_prev:      BoundKey::KeyPageDown,
};

#[test]
fn every_named_action_reads_its_own_row_of_the_keybind_table() {
    for (action, expected, row) in [
        (
            KeybindActionNet::SelectClear,
            BoundKey::KeyQ,
            "select_clear",
        ),
        (KeybindActionNet::LevelUp, BoundKey::KeyE, "level_up"),
        (KeybindActionNet::LevelDown, BoundKey::KeyC, "level_down"),
        (
            KeybindActionNet::ToggleFullView,
            BoundKey::KeyF,
            "toggle_full_view",
        ),
        (
            KeybindActionNet::StanceCycle,
            BoundKey::KeyR,
            "stance_cycle",
        ),
        (KeybindActionNet::AimToggle, BoundKey::KeyV, "aim_toggle"),
        (
            KeybindActionNet::FacingCycle,
            BoundKey::KeyTab,
            "facing_cycle",
        ),
        (
            KeybindActionNet::SelectNext,
            BoundKey::KeyPageUp,
            "select_next",
        ),
        (
            KeybindActionNet::SelectPrev,
            BoundKey::KeyPageDown,
            "select_prev",
        ),
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
        assert_eq!(
            action.bound(&TABLE),
            expected,
            "`{action:?}` must resolve through the table's `{row}`, or `input.press_key` presses \
             a key the client never named",
        );
    }
}
