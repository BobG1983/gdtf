//! Act intent bus: contextual acts, keybinds, keyboard, and intent drain.

/// Contextual act seam and per-act markers.
pub mod contextual;
/// Facing and stance cycle orders.
pub mod cycle;
/// Panel focus bridge for UI button navigation.
pub mod focus_bridge;
/// Pending act intents and dispatch.
pub mod intent;
/// Hot-loaded keybind map.
pub mod keybinds;
/// Keyboard systems that push intents.
pub mod keyboard;
/// Input system sets.
pub mod sets;
