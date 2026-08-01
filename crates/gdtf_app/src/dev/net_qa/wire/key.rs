//! The raw-keyboard wire vocabulary — [`KeyNet`], [`KeybindActionNet`], the composed
//! [`KeyPressNet`] a [`NetIntent::PressKey`](super::act::NetIntent::PressKey) taps
//! (GTW-783), and [`FocusStepNet`], the arrow-key step the focus chain walks (GTW-944).
//!
//! A QA client drives keyboard behaviour two ways, and [`KeyPressNet`] carries either: a
//! named physical [`Key`](KeyPressNet::Key) (a [`KeyNet`], resolved to a Bevy `KeyCode`
//! directly) or a named bound [`Action`](KeyPressNet::Action) (a [`KeybindActionNet`],
//! resolved through the game's live `Keybinds` table to whatever key it is currently on).
//! Both stay INDEPENDENT serde enums — never a re-export of Bevy's `KeyCode` or the input
//! crate's `Keybinds`, so a Bevy rename cannot silently change the wire. The game side maps
//! them back on inject.
//!
//! A caller CHOOSES a key, an action or a focus step; none is handed out by a read. What IS
//! published is the mapping: the C9 `input.keybinds` reports which key each bound action
//! currently sits on, so a client can drive either form knowingly. The C14 `input.press_key`
//! and `input.focus_step` are the commands that take them.

use schemars::JsonSchema;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeyNet {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeybindActionNet {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum KeyPressNet {
    /// Tap a named physical key.
    Key(KeyNet),
    /// Tap whatever key the named bound action is currently on.
    Action(KeybindActionNet),
}

/// One step of focus movement — the wire mirror of an arrow-key / D-pad press.
///
/// A caller CHOOSES a direction; no command publishes one. It is the `direction` argument of
/// the C14 `input.focus_step`, which writes the SAME navigate message an arrow key writes and
/// lets the game's own navigation system move the focus — the command never writes
/// `InputFocus` itself. To point focus at one named control instead, C14's `input.set_focus`
/// takes a [`FocusTargetNet`](super::token::FocusTargetNet), which the C9 `ui.focus`
/// publishes.
///
/// [`Next`](Self::Next) / [`Prev`](Self::Prev) walk the screen's vertical focus chain (the
/// direction the down / up arrow moves); [`Left`](Self::Left) / [`Right`](Self::Right) walk
/// the horizontal one. An independent, closed serde enum — never a leak of the game's own
/// direction type. Deleted from the shared protocol crate with the rest of the old QA API
/// (GTW-943) and re-minted here, where the command that takes it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum FocusStepNet {
    /// Move focus to the next control down the chain (the down arrow / D-pad down).
    Next,
    /// Move focus to the previous control up the chain (the up arrow / D-pad up).
    Prev,
    /// Move focus one control to the left (the left arrow).
    Left,
    /// Move focus one control to the right (the right arrow).
    Right,
}
