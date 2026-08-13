//! Keybind vocabulary and hot-loaded keybind table.

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

/// Named key that can appear in the keybind table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum BoundKey {
    /// Escape.
    KeyEscape,
    /// Q.
    KeyQ,
    /// E.
    KeyE,
    /// C.
    KeyC,
    /// F.
    KeyF,
    /// R.
    KeyR,
    /// V.
    KeyV,
    /// Tab.
    KeyTab,
    /// Page Up.
    KeyPageUp,
    /// Page Down.
    KeyPageDown,
    /// `[`.
    KeyBracketLeft,
    /// `]`.
    KeyBracketRight,
    /// Digit 1.
    KeyDigit1,
    /// Digit 2.
    KeyDigit2,
    /// Digit 3.
    KeyDigit3,
    /// Digit 4.
    KeyDigit4,
    /// Digit 5.
    KeyDigit5,
    /// Digit 6.
    KeyDigit6,
    /// Digit 7.
    KeyDigit7,
    /// Digit 8.
    KeyDigit8,
    /// Digit 9.
    KeyDigit9,
    /// Enter.
    KeyEnter,
    /// Arrow up.
    KeyArrowUp,
    /// Arrow down.
    KeyArrowDown,
    /// Arrow left.
    KeyArrowLeft,
    /// Arrow right.
    KeyArrowRight,
}

impl BoundKey {
    /// Map to a Bevy [`KeyCode`].
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

/// Keybind table. The shipped defaults live in code; the RON overrides them.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Deserialize, TypePath)]
pub struct Keybinds {
    /// Clear selection.
    pub select_clear:     BoundKey,
    /// Step active level up.
    pub level_up:         BoundKey,
    /// Step active level down.
    pub level_down:       BoundKey,
    /// Toggle full-view mode.
    pub toggle_full_view: BoundKey,
    /// Cycle stance.
    pub stance_cycle:     BoundKey,
    /// Toggle aiming.
    pub aim_toggle:       BoundKey,
    /// Cycle facing.
    pub facing_cycle:     BoundKey,
    /// Select next player ganger.
    pub select_next:      BoundKey,
    /// Select previous player ganger.
    pub select_prev:      BoundKey,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            select_clear:     BoundKey::KeyEscape,
            level_up:         BoundKey::KeyPageUp,
            level_down:       BoundKey::KeyPageDown,
            toggle_full_view: BoundKey::KeyV,
            stance_cycle:     BoundKey::KeyC,
            aim_toggle:       BoundKey::KeyF,
            facing_cycle:     BoundKey::KeyR,
            select_next:      BoundKey::KeyTab,
            select_prev:      BoundKey::KeyTab,
        }
    }
}

impl Keybinds {
    /// Key code for clear selection.
    #[must_use]
    pub const fn select_clear(&self) -> KeyCode {
        self.select_clear.key_code()
    }

    /// Key code for level up.
    #[must_use]
    pub const fn level_up(&self) -> KeyCode {
        self.level_up.key_code()
    }

    /// Key code for level down.
    #[must_use]
    pub const fn level_down(&self) -> KeyCode {
        self.level_down.key_code()
    }

    /// Key code for toggle full view.
    #[must_use]
    pub const fn toggle_full_view(&self) -> KeyCode {
        self.toggle_full_view.key_code()
    }

    /// Key code for stance cycle.
    #[must_use]
    pub const fn stance_cycle(&self) -> KeyCode {
        self.stance_cycle.key_code()
    }

    /// Key code for aim toggle.
    #[must_use]
    pub const fn aim_toggle(&self) -> KeyCode {
        self.aim_toggle.key_code()
    }

    /// Key code for facing cycle.
    #[must_use]
    pub const fn facing_cycle(&self) -> KeyCode {
        self.facing_cycle.key_code()
    }

    /// Key code for select next.
    #[must_use]
    pub const fn select_next(&self) -> KeyCode {
        self.select_next.key_code()
    }

    /// Key code for select prev.
    #[must_use]
    pub const fn select_prev(&self) -> KeyCode {
        self.select_prev.key_code()
    }

    /// Digit key for contextual act slot `rank` (1-based), if in range.
    #[must_use]
    pub fn contextual_slot_key(rank: SlotRank) -> Option<KeyCode> {
        let index = (*rank).checked_sub(1)?;
        CONTEXTUAL_SLOT_KEYS
            .get(usize::from(index))
            .map(|key| key.key_code())
    }
}

pub(crate) fn register_keybinds_hot_ron(app: &mut App) {
    // Outside the hot-RON chain: that install returns early with no `AssetServer`.
    app.init_resource::<Keybinds>();
    app.init_hot_ron_resource::<Keybinds>(KEYBINDS_RON_PATH);
}
