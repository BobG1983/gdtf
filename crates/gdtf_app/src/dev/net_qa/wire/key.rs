use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeyNet {
        Escape,
        KeyQ,
        KeyE,
        KeyC,
        KeyF,
        KeyR,
        KeyV,
        Tab,
        PageUp,
        PageDown,
        BracketLeft,
        BracketRight,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeybindActionNet {
        SelectClear,
        LevelUp,
        LevelDown,
        ToggleFullView,
        StanceCycle,
        AimToggle,
        FacingCycle,
        SelectNext,
        SelectPrev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeyPressNet {
        Key(KeyNet),
        Action(KeybindActionNet),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum FocusStepNet {
        Next,
        Prev,
        Left,
        Right,
}
