//! Keyboard and focus step keys on the wire.

use gdtf_battle_input::{BoundKey, Keybinds};
use serde::{Deserialize, Serialize};

/// Physical key on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyNet {
    /// Escape.
    Escape,
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
    Tab,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Left bracket.
    BracketLeft,
    /// Right bracket.
    BracketRight,
    /// Digit 1.
    Digit1,
    /// Digit 2.
    Digit2,
    /// Digit 3.
    Digit3,
    /// Digit 4.
    Digit4,
    /// Digit 5.
    Digit5,
    /// Digit 6.
    Digit6,
    /// Digit 7.
    Digit7,
    /// Digit 8.
    Digit8,
    /// Digit 9.
    Digit9,
    /// Enter.
    Enter,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
}

impl KeyNet {
    /// The keybind vocabulary's name for this key.
    #[must_use]
    pub const fn bound(self) -> BoundKey {
        match self {
            Self::Escape => BoundKey::KeyEscape,
            Self::KeyQ => BoundKey::KeyQ,
            Self::KeyE => BoundKey::KeyE,
            Self::KeyC => BoundKey::KeyC,
            Self::KeyF => BoundKey::KeyF,
            Self::KeyR => BoundKey::KeyR,
            Self::KeyV => BoundKey::KeyV,
            Self::Tab => BoundKey::KeyTab,
            Self::PageUp => BoundKey::KeyPageUp,
            Self::PageDown => BoundKey::KeyPageDown,
            Self::BracketLeft => BoundKey::KeyBracketLeft,
            Self::BracketRight => BoundKey::KeyBracketRight,
            Self::Digit1 => BoundKey::KeyDigit1,
            Self::Digit2 => BoundKey::KeyDigit2,
            Self::Digit3 => BoundKey::KeyDigit3,
            Self::Digit4 => BoundKey::KeyDigit4,
            Self::Digit5 => BoundKey::KeyDigit5,
            Self::Digit6 => BoundKey::KeyDigit6,
            Self::Digit7 => BoundKey::KeyDigit7,
            Self::Digit8 => BoundKey::KeyDigit8,
            Self::Digit9 => BoundKey::KeyDigit9,
            Self::Enter => BoundKey::KeyEnter,
            Self::ArrowUp => BoundKey::KeyArrowUp,
            Self::ArrowDown => BoundKey::KeyArrowDown,
            Self::ArrowLeft => BoundKey::KeyArrowLeft,
            Self::ArrowRight => BoundKey::KeyArrowRight,
        }
    }

    /// The wire's name for a key the keybind table holds.
    #[must_use]
    pub const fn from_bound(key: BoundKey) -> Self {
        match key {
            BoundKey::KeyEscape => Self::Escape,
            BoundKey::KeyQ => Self::KeyQ,
            BoundKey::KeyE => Self::KeyE,
            BoundKey::KeyC => Self::KeyC,
            BoundKey::KeyF => Self::KeyF,
            BoundKey::KeyR => Self::KeyR,
            BoundKey::KeyV => Self::KeyV,
            BoundKey::KeyTab => Self::Tab,
            BoundKey::KeyPageUp => Self::PageUp,
            BoundKey::KeyPageDown => Self::PageDown,
            BoundKey::KeyBracketLeft => Self::BracketLeft,
            BoundKey::KeyBracketRight => Self::BracketRight,
            BoundKey::KeyDigit1 => Self::Digit1,
            BoundKey::KeyDigit2 => Self::Digit2,
            BoundKey::KeyDigit3 => Self::Digit3,
            BoundKey::KeyDigit4 => Self::Digit4,
            BoundKey::KeyDigit5 => Self::Digit5,
            BoundKey::KeyDigit6 => Self::Digit6,
            BoundKey::KeyDigit7 => Self::Digit7,
            BoundKey::KeyDigit8 => Self::Digit8,
            BoundKey::KeyDigit9 => Self::Digit9,
            BoundKey::KeyEnter => Self::Enter,
            BoundKey::KeyArrowUp => Self::ArrowUp,
            BoundKey::KeyArrowDown => Self::ArrowDown,
            BoundKey::KeyArrowLeft => Self::ArrowLeft,
            BoundKey::KeyArrowRight => Self::ArrowRight,
        }
    }
}

/// Named keybind action on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeybindActionNet {
    /// Clear selection.
    SelectClear,
    /// Level up.
    LevelUp,
    /// Level down.
    LevelDown,
    /// Toggle full view.
    ToggleFullView,
    /// Cycle stance.
    StanceCycle,
    /// Toggle aim.
    AimToggle,
    /// Cycle facing.
    FacingCycle,
    /// Select next ganger.
    SelectNext,
    /// Select previous ganger.
    SelectPrev,
}

impl KeybindActionNet {
    /// The key this action is bound to in the live keybind table.
    #[must_use]
    pub const fn bound(self, keybinds: &Keybinds) -> BoundKey {
        match self {
            Self::SelectClear => keybinds.select_clear,
            Self::LevelUp => keybinds.level_up,
            Self::LevelDown => keybinds.level_down,
            Self::ToggleFullView => keybinds.toggle_full_view,
            Self::StanceCycle => keybinds.stance_cycle,
            Self::AimToggle => keybinds.aim_toggle,
            Self::FacingCycle => keybinds.facing_cycle,
            Self::SelectNext => keybinds.select_next,
            Self::SelectPrev => keybinds.select_prev,
        }
    }
}

/// Either a physical key or a named keybind action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyPressNet {
    /// Physical key.
    Key(KeyNet),
    /// Named action.
    Action(KeybindActionNet),
}

/// Focus navigation step on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusStepNet {
    /// Next focus target.
    Next,
    /// Previous focus target.
    Prev,
    /// Move focus left.
    Left,
    /// Move focus right.
    Right,
}
