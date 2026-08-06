//! Turning a wire key press into the keyboard messages `bevy_winit` would have written.

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
};
use gdtf_battle_input::Keybinds;

use crate::dev::net_qa::wire::key::{KeyNet, KeyPressNet};

/// The physical key a press names, resolving an action through the live keybind table.
pub(super) fn resolve(press: KeyPressNet, keybinds: Option<&Keybinds>) -> Option<KeyNet> {
    match press {
        KeyPressNet::Key(key) => Some(key),
        KeyPressNet::Action(action) => {
            keybinds.map(|table| KeyNet::from_bound(action.bound(table)))
        }
    }
}

/// The message `keyboard_input_system` folds into `ButtonInput<KeyCode>` next frame.
pub(super) fn key_message(key: KeyNet, state: ButtonState) -> KeyboardInput {
    KeyboardInput {
        key_code: key.bound().key_code(),
        logical_key: logical_key(key),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

fn logical_key(key: KeyNet) -> Key {
    match key {
        KeyNet::Escape => Key::Escape,
        KeyNet::KeyQ => Key::Character("q".into()),
        KeyNet::KeyE => Key::Character("e".into()),
        KeyNet::KeyC => Key::Character("c".into()),
        KeyNet::KeyF => Key::Character("f".into()),
        KeyNet::KeyR => Key::Character("r".into()),
        KeyNet::KeyV => Key::Character("v".into()),
        KeyNet::Tab => Key::Tab,
        KeyNet::PageUp => Key::PageUp,
        KeyNet::PageDown => Key::PageDown,
        KeyNet::BracketLeft => Key::Character("[".into()),
        KeyNet::BracketRight => Key::Character("]".into()),
        KeyNet::Digit1 => Key::Character("1".into()),
        KeyNet::Digit2 => Key::Character("2".into()),
        KeyNet::Digit3 => Key::Character("3".into()),
        KeyNet::Digit4 => Key::Character("4".into()),
        KeyNet::Digit5 => Key::Character("5".into()),
        KeyNet::Digit6 => Key::Character("6".into()),
        KeyNet::Digit7 => Key::Character("7".into()),
        KeyNet::Digit8 => Key::Character("8".into()),
        KeyNet::Digit9 => Key::Character("9".into()),
        KeyNet::Enter => Key::Enter,
        KeyNet::ArrowUp => Key::ArrowUp,
        KeyNet::ArrowDown => Key::ArrowDown,
        KeyNet::ArrowLeft => Key::ArrowLeft,
        KeyNet::ArrowRight => Key::ArrowRight,
    }
}
