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
use gdtf_battle_sim::{ganger::GangName, level::ThemeUuid};
use gdtf_ui::{register_dropdown, register_numeric_field};

use crate::states::{
    RunningState,
    running::procgen_viz::{
        config::{
            apply::{
                apply_gang_selection, apply_seed_commit, apply_size_commit, apply_theme_selection,
                generate_on_press, sync_size_status,
            },
            resource::VizConfig,
        },
        model::ProcgenViz,
        systems::{
            auto_on_press, insert_viz_model, remove_viz_model, respawn_quads_on_generate,
            spawn_viz_screen, step_on_press, sync_revealed_quads,
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
        // GTW-498: register the input widgets' per-`T` / per-`N` drivers ONCE (an unregistered
        // generic widget is a dead feature — the editor-plugin precedent). The theme + gang
        // dropdowns are `Dropdown<ThemeUuid>` / `Dropdown<GangName>`; the grid-axis fields are
        // `NumericField<u8>` and the seed field `NumericField<u64>`.
        register_dropdown::<ThemeUuid>(app);
        register_dropdown::<GangName>(app);
        register_numeric_field::<u8>(app);
        register_numeric_field::<u64>(app);

        // OnEnter: build + insert the model + the config BEFORE spawning the screen (so the
        // spawn reads both, C6).
        app.add_systems(
            OnEnter(RunningState::DebugProcgenVisualizer),
            (insert_viz_model, spawn_viz_screen).chain(),
        );

        // Update — the input panel + Generate (GTW-498), ordered before the reveal chain so a
        // same-frame Generate rebuilds + re-spawns the quads BEFORE the draw sync reveals them:
        //   1. apply_* listeners mutate VizConfig from the real widget messages (C1–C4);
        //   2. sync_size_status surfaces an invalid size + disables Generate (C2);
        //   3. generate_on_press rebuilds the model from the config + resets the reveal (C5);
        //   4. respawn_quads_on_generate re-spawns the per-prefab quads for the NEW model (C5).
        // Gated on the state + the config resource (bevy-traps #1).
        app.add_systems(
            Update,
            (
                (
                    apply_theme_selection,
                    apply_gang_selection,
                    apply_size_commit,
                    apply_seed_commit,
                ),
                sync_size_status,
                generate_on_press,
                respawn_quads_on_generate,
            )
                .chain()
                .run_if(
                    in_state(RunningState::DebugProcgenVisualizer)
                        .and_then(resource_exists::<VizConfig>)
                        .and_then(resource_exists::<ProcgenViz>),
                ),
        );

        // Update: the STEP / AUTO controls advance the model, then the draw layer reveals the
        // matching quads — ordered so a same-frame press is reflected the same frame
        // (bevy-traps #3). Ordered AFTER the GTW-498 Generate chain so a same-frame Generate's
        // new quads are revealed by the SAME sync. Gated on the state AND the model resource
        // existing (bevy-traps #1).
        app.add_systems(
            Update,
            ((step_on_press, auto_on_press), sync_revealed_quads)
                .chain()
                .after(respawn_quads_on_generate)
                .run_if(
                    in_state(RunningState::DebugProcgenVisualizer)
                        .and_then(resource_exists::<ProcgenViz>),
                ),
        );

        // OnExit: remove the model + the config resources (the screen entities self-despawn via
        // DespawnOnExit).
        app.add_systems(
            OnExit(RunningState::DebugProcgenVisualizer),
            (remove_viz_model, remove_viz_config),
        );

        // DEV-ONLY QA hook: the visualizer self-screenshot, double-gated on the dev cfg + its
        // own env var (the GTW-419 / GTW-420 capture discipline). Inert in a normal build.
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        super::capture::register_viz_capture(app);
    }
}

/// Remove the [`VizConfig`] state-scoped resource `OnExit(DebugProcgenVisualizer)` (the
/// state-scoped-resource convention, `bevy-traps.md` #1). The screen entities tear down via
/// their own `DespawnOnExit` markers. Param-only (`bevy-traps.md` #7).
fn remove_viz_config(mut commands: Commands) {
    commands.remove_resource::<VizConfig>();
}
