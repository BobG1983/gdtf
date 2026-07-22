//! [`SceneLabel`] — the human-readable scene name the enter/exit loggers print.

use bevy::prelude::Deref;

/// The scene name printed by the [`log_scene_enter`](super::log_scene_enter) /
/// [`log_scene_exit`](super::log_scene_exit) loggers — e.g. `"Init"` or
/// `"Game::BattleScape::AfterMath::AnimateIn"`.
///
/// A named newtype (no-bare-types): the label is a domain value — it must keep
/// every scene's enter/exit log line per-scene distinguishable — so it never
/// travels as a bare `&str`. `'static` because labels are compile-time scene
/// constants named at plugin registration.
#[derive(Deref, Clone, Copy, Debug)]
pub(in crate::states) struct SceneLabel(&'static str);

impl SceneLabel {
    /// Wraps a scene's display name (the exact text the scene's old
    /// hand-stamped `print_state.rs` printed, so the log lines stay
    /// identical across the GTW-575 sweep).
    pub(in crate::states) const fn new(label: &'static str) -> Self {
        Self(label)
    }
}
