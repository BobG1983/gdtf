//! The raw-keyboard wire vocabulary — [`KeyNet`], [`KeybindActionNet`] and the composed
//! [`KeyPressNet`] a [`NetIntent::PressKey`](super::act::NetIntent::PressKey) taps (GTW-783).
//!
//! A QA client drives keyboard behaviour two ways, and [`KeyPressNet`] carries either: a
//! named physical [`Key`](KeyPressNet::Key) (a [`KeyNet`], resolved to a Bevy `KeyCode`
//! directly) or a named bound [`Action`](KeyPressNet::Action) (a [`KeybindActionNet`],
//! resolved through the game's live `Keybinds` table to whatever key it is currently on).
//! Both stay INDEPENDENT serde enums — never a leak of Bevy's `KeyCode` or the input
//! crate's `Keybinds` (this crate is bevy-free). The game side maps them back on inject.

use serde::{Deserialize, Serialize};

/// A named **physical key** — the wire mirror of the Bevy `KeyCode` subset the game reads.
///
/// An independent serde enum whose variant names track Bevy's `KeyCode` for recognisability
/// (`Tab`, `Escape`, `ArrowLeft`, `Digit1`). The set is the keyboard the game already binds
/// (the input crate's `BoundKey` vocabulary — Escape / the act letters / Tab / `PageUp` /
/// `PageDown` / the brackets / the nine slot digits) PLUS the four arrow keys the focus-
/// navigation bridge reads (GTW-782's `ArrowLeft`/`ArrowRight` → directional focus). Extend
/// it as later features read more keys — the same extend-as-needed philosophy the game's own
/// `BoundKey` documents. The game side maps each variant to its `KeyCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum KeyNet {
    /// The `Escape` key.
    Escape,
    /// The `Q` key.
    KeyQ,
    /// The `E` key.
    KeyE,
    /// The `C` key.
    KeyC,
    /// The `F` key.
    KeyF,
    /// The `R` key.
    KeyR,
    /// The `V` key.
    KeyV,
    /// The `Tab` key.
    Tab,
    /// The `PageUp` key.
    PageUp,
    /// The `PageDown` key.
    PageDown,
    /// The bracket-left `[` key.
    BracketLeft,
    /// The bracket-right `]` key.
    BracketRight,
    /// The `1` digit key.
    Digit1,
    /// The `2` digit key.
    Digit2,
    /// The `3` digit key.
    Digit3,
    /// The `4` digit key.
    Digit4,
    /// The `5` digit key.
    Digit5,
    /// The `6` digit key.
    Digit6,
    /// The `7` digit key.
    Digit7,
    /// The `8` digit key.
    Digit8,
    /// The `9` digit key.
    Digit9,
    /// The up-arrow key (directional focus navigation).
    ArrowUp,
    /// The down-arrow key (directional focus navigation).
    ArrowDown,
    /// The left-arrow key (directional focus navigation).
    ArrowLeft,
    /// The right-arrow key (directional focus navigation).
    ArrowRight,
}

/// A named **bound action** — the wire mirror of the game's `Keybinds` table fields.
///
/// An independent serde enum, one variant per bound act. A
/// [`KeyPress`](KeyPressNet::Action) naming an action taps whatever key the live `Keybinds`
/// table currently binds it to (so a QA client can exercise the keyboard path without
/// knowing the concrete key). The game side reads the matching `Keybinds` accessor to
/// resolve the `KeyCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum KeybindActionNet {
    /// Clear the current selection (`Keybinds::select_clear`).
    SelectClear,
    /// Raise the presenter's active level one storey (`Keybinds::level_up`).
    LevelUp,
    /// Lower the presenter's active level one storey (`Keybinds::level_down`).
    LevelDown,
    /// Toggle the presenter's full-stack view (`Keybinds::toggle_full_view`).
    ToggleFullView,
    /// Step the selected ganger's stance (`Keybinds::stance_cycle`).
    StanceCycle,
    /// Toggle the selected ganger's aim mode (`Keybinds::aim_toggle`).
    AimToggle,
    /// Step the selected ganger's facing (`Keybinds::facing_cycle`).
    FacingCycle,
    /// Cycle the selection to the next player ganger (`Keybinds::select_next`).
    SelectNext,
    /// Cycle the selection to the previous player ganger (`Keybinds::select_prev`).
    SelectPrev,
}

/// One key tap a [`NetIntent::PressKey`](super::act::NetIntent::PressKey) simulates — named
/// either by a physical [`Key`](Self::Key) or by a bound [`Action`](Self::Action).
///
/// The game side resolves either form to a `KeyCode` and writes a real `KeyboardInput`
/// press+release pair through the same message stream the windowing backend feeds — never a
/// direct poke of `ButtonInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum KeyPressNet {
    /// Tap a named physical key.
    Key(KeyNet),
    /// Tap whatever key the named bound action is currently on.
    Action(KeybindActionNet),
}
