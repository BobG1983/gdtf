//! Test-support surface for building and asserting on the real GDTF state
//! machine from an external crate.
//!
//! This module is gated behind the `test-support` feature and is absent from
//! release builds — the production public API of `gdtf_app` is exactly
//! [`crate::GdtfApp`]. It re-exports the otherwise crate-internal state enums
//! and [`ScenesPlugin`] so an out-of-crate harness can name them, and provides
//! [`register_headless`] to wire the full headless state stack in the correct
//! parent-before-child order.

use bevy::{
    app::App,
    state::app::{AppExtStates, StatesPlugin},
};
pub use gdtf_ui::UiPlugin;

pub use crate::{
    scenes::ScenesPlugin,
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

/// Registers the full headless GDTF state stack on `app`.
///
/// Wires, in order:
/// 1. [`StatesPlugin`] — installs the `StateTransition` schedule the state
///    machinery needs (normally pulled in by `DefaultPlugins`, which a headless
///    test does not add).
/// 2. [`AppState`] as the top-level state.
/// 3. The sub-states in parent-before-child order — [`RunningState`],
///    [`GameState`], [`BattleScapeState`], [`AfterMathState`] — because a
///    `SubStates` type must be registered after its `#[source(...)]` parent (see
///    `.claude/rules/bevy-traps.md` rule 5).
/// 4. [`ScenesPlugin`], which adds every scene plugin.
/// 5. [`UiPlugin`], the UI registration seam — added here to keep this headless
///    path a faithful mirror of [`crate::GdtfApp`], which also adds it. A
///    harness test can then assert `is_plugin_added::<UiPlugin>()` and prove the
///    real registration path wires the UI, not merely that `gdtf_ui` compiles.
///
/// This intentionally does **not** add `MinimalPlugins`; the headless app
/// builder composes those around `register_headless`. `init_state` /
/// `add_sub_state` are idempotent in Bevy 0.18, so the sub-state registrations
/// that `ScenesPlugin`'s scene plugins also perform are a no-op the second time.
pub fn register_headless(app: &mut App) {
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_sub_state::<RunningState>();
    app.add_sub_state::<GameState>();
    app.add_sub_state::<BattleScapeState>();
    app.add_sub_state::<AfterMathState>();
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}
