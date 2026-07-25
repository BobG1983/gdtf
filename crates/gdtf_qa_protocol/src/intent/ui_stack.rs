//! [`UiStackNet`] — the wire name of one UI stack under live comparison (GTW-816).
//!
//! The game can render the same screen through either of two UI stacks and swap which one
//! is live at runtime (the DEV swap harness). This enum is the wire mirror of the game's
//! own stack identifier: the payload of a
//! [`NetIntent::SwapUiStack`](super::NetIntent::SwapUiStack), and the value the
//! [`UiStackView`](crate::view::UiStackView) reports back in the battle snapshot.
//!
//! Kept out of [`payload`](super::payload) — that file holds the sim-value mirrors
//! (posture / facing / melee target); a UI stack is not a sim value.

use serde::{Deserialize, Serialize};

/// One UI stack, by name — the wire mirror of the game's own stack identifier.
///
/// Two stacks are under comparison: Bevy's retained-mode `bevy_ui` (the shipping game
/// HUD) and the immediate-mode egui stack (the content editor's, under evaluation for the
/// game). An independent serde enum, like every other wire mirror in this crate — the
/// protocol never names an engine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UiStackNet {
    /// Bevy's retained-mode `bevy_ui` — the stack the shipping HUD is built in.
    BevyUi,
    /// The immediate-mode egui stack.
    Egui,
}
