//! [`EditorScenePlugin`] — the in-app gang-editor scene (GTW-420), the foundation of the
//! GTW-403 gang-editor track.
//!
//! Registered by [`RunningScenePlugin`](super::super::plugin::RunningScenePlugin) like the
//! other [`RunningState`] scenes. It wires the editor screen + its editable model around
//! [`RunningState::DebugEditor`]:
//!
//! - `OnEnter(DebugEditor)`: [`insert_editable_gang`] inserts the
//!   [`EditableGang`](super::model::EditableGang) model (loaded
//!   from the [`GangRegistry`](gdtf_battle_sim::GangRegistry), or empty — AC2), ORDERED BEFORE
//!   [`spawn_editor_screen`] which builds the themed panel layout (the gang-name field via
//!   [`spawn_text_field`](gdtf_ui::spawn_text_field), the "Add member" button, the member-list
//!   shell).
//! - `Update` (gated `in_state(DebugEditor)`): [`commit_gang_name`] (AC3) +
//!   [`add_member_on_press`] (AC4).
//! - `OnExit(DebugEditor)`: [`remove_editable_gang`] removes the model resource (C1); the screen
//!   entities tear down via their own `DespawnOnExit` markers.
//!
//! It ALSO calls [`register_text_field`](gdtf_ui::register_text_field) once, wiring the GTW-411
//! text-field widget's type-agnostic systems + commit observers so the gang-name field edits and
//! commits. The editor is the first (and only) consumer of the text-field widget in the app.

use bevy::prelude::*;
use gdtf_ui::register_text_field;

use crate::states::{RunningState, running::editor::systems::*};

/// Wires the gang-editor scene plugin (GTW-420).
pub(in crate::states) struct EditorScenePlugin;

impl Plugin for EditorScenePlugin {
    fn build(&self, app: &mut App) {
        // The GTW-411 text-field widget's type-agnostic systems + commit observers. The editor
        // is the only consumer of the text field today, so it owns this one-time registration.
        register_text_field(app);

        // OnEnter: insert the model BEFORE the screen is spawned (so the spawn sees it).
        app.add_systems(
            OnEnter(RunningState::DebugEditor),
            (insert_editable_gang, spawn_editor_screen).chain(),
        );

        // Update: the gang-name edit (AC3) + add-member (AC4) effects, gated on the editor state
        // and (within each system) the model's presence.
        app.add_systems(
            Update,
            (commit_gang_name, add_member_on_press).run_if(in_state(RunningState::DebugEditor)),
        );

        // OnExit: remove the model resource (the screen entities self-despawn via DespawnOnExit).
        app.add_systems(OnExit(RunningState::DebugEditor), remove_editable_gang);

        // DEV-ONLY QA hook: the editor self-screenshot, double-gated on the dev cfg + its own env
        // var (the GTW-419 / GTW-297 capture discipline). Inert in a normal build.
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        super::capture::register_editor_capture(app);
    }
}
