//! [`GangEditorScenePlugin`] — the in-app gang-editor scene (GTW-420), the foundation of the
//! GTW-403 gang-editor track.
//!
//! Registered by [`RunningScenePlugin`](super::super::plugin::RunningScenePlugin) like the
//! other [`RunningState`] scenes. It wires the editor screen + its editable model around
//! [`RunningState::DebugGangEditor`]:
//!
//! - `OnEnter(DebugGangEditor)`: [`insert_editable_gang`] inserts the
//!   [`EditableGang`](super::model::EditableGang) model (loaded
//!   from the [`GangRegistry`](gdtf_battle_sim::GangRegistry), or empty — AC2), ORDERED BEFORE
//!   [`spawn_editor_screen`] which builds the themed panel layout (the gang-name field via
//!   [`spawn_text_field`](gdtf_ui::spawn_text_field), the "Add member" button, the member-list
//!   shell).
//! - `Update` (gated `in_state(DebugGangEditor)`): [`commit_gang_name`] (AC3), [`add_member_on_press`]
//!   (AC4), the per-member inline edits [`commit_member_name`] (GTW-425 C3) /
//!   [`commit_member_weapon`] / [`commit_member_armor`] (GTW-425 C2), the base-attribute edit +
//!   live derived recompute [`commit_member_attribute`] (GTW-428 C2/C3),
//!   [`delete_member_on_press`] (C4), and the expand-pip→accordion [`toggle_expand_pip`]
//!   (GTW-428 C1). The GTW-416 accordion height-lerp itself rides `UiPlugin`'s `drive_accordions`.
//!   In a DEBUG build it ALSO registers `save_gang_on_press` (GTW-429 C4): a press on the "Save
//!   gang" button writes the edited model to `assets/content/gangs/<gang_name>.gang.ron` in the GTW-415
//!   schema (C1). The save system + its fs-write are `cfg(debug_assertions)`-gated (C3).
//! - `OnExit(DebugGangEditor)`: [`remove_editable_gang`] removes the model resource (C1); the screen
//!   entities tear down via their own `DespawnOnExit` markers.
//!
//! It ALSO calls [`register_text_field`](gdtf_ui::register_text_field) once (the GTW-411 text-field
//! widget's type-agnostic systems + commit observers, for the gang-name + per-member name fields),
//! [`register_numeric_field`](gdtf_ui::register_numeric_field)`::<f32>` once (the GTW-411 numeric
//! field's f32 commit / revert handlers + message, for the eight editable base-attribute fields —
//! GTW-428 C2), and [`register_dropdown`](gdtf_ui::register_dropdown)`::<WeaponName>` /
//! `::<ArmorName>` once each (the GTW-410 dropdown drivers + selection messages for the per-member
//! weapon / armor selectors — C2). The editor is the first consumer of all three widgets in the app.

use bevy::prelude::*;
use gdtf_battle_sim::{ArmorName, WeaponName};
use gdtf_ui::{register_dropdown, register_numeric_field, register_text_field};

use crate::states::{RunningState, running::gang_editor::systems::*};

/// Wires the gang-editor scene plugin (GTW-420).
pub(in crate::states) struct GangEditorScenePlugin;

impl Plugin for GangEditorScenePlugin {
    fn build(&self, app: &mut App) {
        // The GTW-411 text-field widget's type-agnostic systems + commit observers (the gang-name
        // field + per-member name fields). The editor is the only consumer of the text field
        // today, so it owns this one-time registration.
        register_text_field(app);
        // The GTW-411 NUMERIC-field's per-`N` pieces for the eight editable base-attribute fields
        // (GTW-428 C2): the `NumericFieldCommitted<f32>` message + the f32-typed commit / revert
        // handlers the type-erased keyboard observer dispatches to. Without this the attribute
        // fields' Enter / blur commits never fire (an unregistered numeric `N` is a dead feature).
        register_numeric_field::<f32>(app);
        // The GTW-410 dropdown drivers + selection messages for the per-member weapon / armor
        // selectors (one registration per option-identity type — C2). Without these the dropdown
        // open/select systems never run (an unregistered widget is a dead feature).
        register_dropdown::<WeaponName>(app);
        register_dropdown::<ArmorName>(app);

        // OnEnter: insert the model BEFORE the screen is spawned (so the spawn sees it).
        app.add_systems(
            OnEnter(RunningState::DebugGangEditor),
            (insert_editable_gang, spawn_editor_screen).chain(),
        );

        // Update: the gang-name edit (AC3), add-member (AC4), the per-member inline edits
        // (name / weapon / armor — GTW-425 C2/C3), the base-attribute edit + live derived recompute
        // (GTW-428 C2/C3), delete (C4), and the expand-pip→accordion toggle (GTW-428 C1) — all gated
        // on the editor state and (within each system) the model's presence.
        app.add_systems(
            Update,
            (
                commit_gang_name,
                add_member_on_press,
                commit_member_name,
                commit_member_weapon,
                commit_member_armor,
                commit_member_attribute,
                delete_member_on_press,
                toggle_expand_pip,
            )
                .run_if(in_state(RunningState::DebugGangEditor)),
        );

        // GTW-429 C4: the SAVE trigger — a press on the "Save gang" button writes the edited model
        // to `assets/content/gangs/<gang_name>.gang.ron` (C1). `#[cfg(debug_assertions)]`-gated (C3) so
        // the filesystem-write system is never compiled into a release binary. Registered as its own
        // gated `add_systems` so the always-compiled tuple above stays release-buildable.
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            save_gang_on_press.run_if(in_state(RunningState::DebugGangEditor)),
        );

        // OnExit: remove the model resource (the screen entities self-despawn via DespawnOnExit).
        app.add_systems(OnExit(RunningState::DebugGangEditor), remove_editable_gang);

        // DEV-ONLY QA hook: the editor self-screenshot, double-gated on the dev cfg + its own env
        // var (the GTW-419 / GTW-297 capture discipline). Inert in a normal build.
        #[cfg(all(debug_assertions, feature = "dev_capture"))]
        super::capture::register_editor_capture(app);
    }
}
