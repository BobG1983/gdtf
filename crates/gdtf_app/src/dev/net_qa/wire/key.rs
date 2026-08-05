//! Keyboard and focus step keys on the wire.

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
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
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
