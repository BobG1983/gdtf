//! The authored key vocabulary, the resolved keybind table, and its hot-RON
use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

use crate::contextual::SlotRank;

const KEYBINDS_RON_PATH: &str = "core_tuning/keybinds.tuning.ron";

const CONTEXTUAL_SLOT_KEYS: [BoundKey; 9] = [
    BoundKey::KeyDigit1,
    BoundKey::KeyDigit2,
    BoundKey::KeyDigit3,
    BoundKey::KeyDigit4,
    BoundKey::KeyDigit5,
    BoundKey::KeyDigit6,
    BoundKey::KeyDigit7,
    BoundKey::KeyDigit8,
    BoundKey::KeyDigit9,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum BoundKey {
        KeyEscape,
        KeyQ,
        KeyE,
        KeyC,
        KeyF,
        KeyR,
        KeyV,
        KeyTab,
        KeyPageUp,
        KeyPageDown,
        KeyBracketLeft,
        KeyBracketRight,
        KeyDigit1,
        KeyDigit2,
        KeyDigit3,
        KeyDigit4,
        KeyDigit5,
        KeyDigit6,
        KeyDigit7,
        KeyDigit8,
        KeyDigit9,
                KeyEnter,
                KeyArrowUp,
            KeyArrowDown,
            KeyArrowLeft,
            KeyArrowRight,
}

impl BoundKey {
                            #[must_use]
    pub const fn key_code(self) -> KeyCode {
        match self {
            Self::KeyEscape => KeyCode::Escape,
            Self::KeyQ => KeyCode::KeyQ,
            Self::KeyE => KeyCode::KeyE,
            Self::KeyC => KeyCode::KeyC,
            Self::KeyF => KeyCode::KeyF,
            Self::KeyR => KeyCode::KeyR,
            Self::KeyV => KeyCode::KeyV,
            Self::KeyTab => KeyCode::Tab,
            Self::KeyPageUp => KeyCode::PageUp,
            Self::KeyPageDown => KeyCode::PageDown,
            Self::KeyBracketLeft => KeyCode::BracketLeft,
            Self::KeyBracketRight => KeyCode::BracketRight,
            Self::KeyDigit1 => KeyCode::Digit1,
            Self::KeyDigit2 => KeyCode::Digit2,
            Self::KeyDigit3 => KeyCode::Digit3,
            Self::KeyDigit4 => KeyCode::Digit4,
            Self::KeyDigit5 => KeyCode::Digit5,
            Self::KeyDigit6 => KeyCode::Digit6,
            Self::KeyDigit7 => KeyCode::Digit7,
            Self::KeyDigit8 => KeyCode::Digit8,
            Self::KeyDigit9 => KeyCode::Digit9,
            Self::KeyEnter => KeyCode::Enter,
            Self::KeyArrowUp => KeyCode::ArrowUp,
            Self::KeyArrowDown => KeyCode::ArrowDown,
            Self::KeyArrowLeft => KeyCode::ArrowLeft,
            Self::KeyArrowRight => KeyCode::ArrowRight,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Deserialize, TypePath)]
pub struct Keybinds {
        pub select_clear:     BoundKey,
        pub level_up:         BoundKey,
        pub level_down:       BoundKey,
                    pub toggle_full_view: BoundKey,
        pub stance_cycle:     BoundKey,
        pub aim_toggle:       BoundKey,
        pub facing_cycle:     BoundKey,
                    pub select_next:      BoundKey,
                        pub select_prev:      BoundKey,
}

impl Keybinds {
        #[must_use]
    pub const fn select_clear(&self) -> KeyCode {
        self.select_clear.key_code()
    }

        #[must_use]
    pub const fn level_up(&self) -> KeyCode {
        self.level_up.key_code()
    }

        #[must_use]
    pub const fn level_down(&self) -> KeyCode {
        self.level_down.key_code()
    }

        #[must_use]
    pub const fn toggle_full_view(&self) -> KeyCode {
        self.toggle_full_view.key_code()
    }

        #[must_use]
    pub const fn stance_cycle(&self) -> KeyCode {
        self.stance_cycle.key_code()
    }

        #[must_use]
    pub const fn aim_toggle(&self) -> KeyCode {
        self.aim_toggle.key_code()
    }

        #[must_use]
    pub const fn facing_cycle(&self) -> KeyCode {
        self.facing_cycle.key_code()
    }

        #[must_use]
    pub const fn select_next(&self) -> KeyCode {
        self.select_next.key_code()
    }

            #[must_use]
    pub const fn select_prev(&self) -> KeyCode {
        self.select_prev.key_code()
    }

                                            #[must_use]
    pub fn contextual_slot_key(rank: SlotRank) -> Option<KeyCode> {
        let index = (*rank).checked_sub(1)?;
        CONTEXTUAL_SLOT_KEYS
            .get(usize::from(index))
            .map(|key| key.key_code())
    }
}

pub(crate) fn register_keybinds_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<Keybinds>(KEYBINDS_RON_PATH);
}
