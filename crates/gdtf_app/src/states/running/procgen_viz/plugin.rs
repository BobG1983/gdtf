//! [`ProcgenVizScenePlugin`] — the DEV-ONLY procgen STEP/AUTO visualizer scene (GTW-434).
//!
//! Registered by [`RunningScenePlugin`](super::super::plugin::RunningScenePlugin) like the
//! other [`RunningState`] scenes, but ONLY under `#[cfg(debug_assertions)]` (C4) — the whole
//! module compiles out of a release build. It wires the visualizer screen + its model around
//! [`RunningState::DebugProcgenVisualizer`]:
//!
//! - `OnEnter(DebugProcgenVisualizer)`: [`insert_viz_model`] runs the sim space-packing
//!   pipeline + inserts the [`ProcgenViz`](super::model::ProcgenViz) model, ORDERED BEFORE
//!   [`spawn_viz_screen`] which builds the dark board quad + the (hidden) per-prefab light
//!   quads + the STEP / AUTO control bar (C1/C2/C3).
//! - `Update` (gated `in_state(DebugProcgenVisualizer)` + `resource_exists::<ProcgenViz>`):
//!   [`step_on_press`] (reveal one — C1) and [`auto_on_press`] (reveal all — C1) advance the
//!   model, then [`sync_revealed_quads`] (ordered `.after` both) reveals the matching quads
//!   (C2). The explicit ordering (bevy-traps #3) means a press THIS frame shows its quad the
//!   SAME frame.
//! - `OnExit(DebugProcgenVisualizer)`: [`remove_viz_model`] removes the model resource (the
//!   screen entities self-despawn via their `DespawnOnExit` markers).
//!
//! A DEV-ONLY self-screenshot QA hook is wired only under `cfg(all(debug_assertions, feature
//! = "dev_capture"))` (the editor-capture discipline) so QA can drive + capture the visualizer
//! mid-run (C5).

use bevy::prelude::*;

use crate::states::{
    RunningState,
    running::procgen_viz::{
        model::ProcgenViz,
        systems::{
            auto_on_press, insert_viz_model, remove_viz_model, spawn_viz_screen, step_on_press,
            sync_revealed_quads,
        },
    },
};

/// Wires the DEV-ONLY procgen-visualizer scene plugin (GTW-434).
///
/// Constructed + added to the app ONLY under `#[cfg(debug_assertions)]` by
/// [`RunningScenePlugin`](super::super::plugin::RunningScenePlugin) (C4).
pub(in crate::states) struct ProcgenVizScenePlugin;

impl Plugin for ProcgenVizScenePlugin {
    fn build(&self, app: &mut App) {
        // OnEnter: build + insert the model BEFORE spawning the screen (so the spawn reads it).
        app.add_systems(
            OnEnter(RunningState::DebugProcgenVisualizer),
            (insert_viz_model, spawn_viz_screen).chain(),
        );

        // Update: the STEP / AUTO controls advance the model, then the draw layer reveals the
        // matching quads — ordered so a same-frame press is reflected the same frame
        // (bevy-traps #3). Gated on the state AND the model resource existing (bevy-traps #1).
        app.add_systems(
            Update,
            ((step_on_press, auto_on_press), sync_revealed_quads)
                .chain()
                .run_if(
                    in_state(RunningState::DebugProcgenVisualizer)
                        .and_then(resource_exists::<ProcgenViz>),
                ),
        );

        // OnExit: remove the model resource (the screen entities self-despawn via DespawnOnExit).
        app.add_systems(
            OnExit(RunningState::DebugProcgenVisualizer),
            remove_viz_model,
        );

        // DEV-ONLY QA hook: the visualizer self-screenshot, double-gated on the dev cfg + its
        // own env var (the GTW-419 / GTW-420 capture discipline). Inert in a normal build.
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        super::capture::register_viz_capture(app);
    }
}
