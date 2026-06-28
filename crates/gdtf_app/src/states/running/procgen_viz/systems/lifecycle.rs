//! The DEV-ONLY procgen-visualizer model lifecycle (GTW-434) — remove the [`ProcgenViz`]
//! model `OnExit(DebugProcgenVisualizer)`.
//!
//! The screen entities self-despawn via their [`DespawnOnExit`](bevy::prelude::DespawnOnExit)
//! markers; this removes the state-scoped model RESOURCE (Bevy has no built-in state-scoped
//! resource — `bevy-traps.md` #1), the [`remove_editable_gang`] precedent. The whole module is
//! `#[cfg(debug_assertions)]`-gated by its parent.
//!
//! [`remove_editable_gang`]: crate::states::running::editor

use bevy::prelude::*;

use crate::states::running::procgen_viz::model::ProcgenViz;

/// Remove the [`ProcgenViz`] model resource `OnExit(DebugProcgenVisualizer)` (the
/// state-scoped-resource convention). The screen entities tear down via their own
/// `DespawnOnExit` markers. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn remove_viz_model(mut commands: Commands) {
    commands.remove_resource::<ProcgenViz>();
}
