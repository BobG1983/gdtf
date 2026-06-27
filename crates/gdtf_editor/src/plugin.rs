//! The [`MapEditorPlugin`] — the editor's single registration seam.
//!
//! Wires the editor's own [`EditorState`] machine, its slim `Load` asset pass
//! ([`register_load`](crate::load::register_load)), the [`Editing`](EditorState::Editing)
//! scene that spawns the four empty themed regions
//! ([`spawn_editor_shell`](crate::regions::spawn_editor_shell)), and the GTW-421 right-panel
//! controls (the theme dropdown + size selector) that populate the
//! [`RightPanelRegion`](crate::RightPanelRegion) and drive the shared [`MapEditorSession`].
//! It does NOT register any of the game's scene plugins or the battle sim runtime (the
//! GTW-417 housing constraint).

use bevy::prelude::*;
use gdtf_battle_sim::level::LevelTheme;
use gdtf_ui::{register_dropdown, register_numeric_field, register_text_field};

use crate::{
    EditorState,
    load::register_load,
    regions::spawn_editor_shell,
    right_panel::{
        GridSpanInput, apply_size_commit, apply_theme_selection, spawn_right_panel_controls,
    },
    session::MapEditorSession,
};

/// The map editor's single plugin: state machine + `Load` pass + `Editing` scene + right
/// panel controls.
///
/// Added by [`MapEditorApp`](crate::MapEditorApp) (and by the headless test) onto an app
/// that already has `DefaultPlugins` + the `gdtf_ui` `UiPlugin`. It owns:
///
/// - `init_state::<EditorState>()` — the editor's own two-state lifecycle.
/// - the slim `Load` pass (theme + weapon/armor/theme-catalog registries) — mirrors the
///   game's `resolve_*` loaders WITHOUT pulling the game scene graph.
/// - the GTW-410 dropdown + GTW-411 numeric-field widget wiring for the editor's own option /
///   value types: [`register_dropdown::<LevelTheme>`], plus [`register_text_field`] (the
///   one-time text-field seam [`register_numeric_field`] requires) and
///   [`register_numeric_field::<GridSpanInput>`] for the three size fields (gate 4b — no
///   unwired per-type system).
/// - `OnEnter(Editing)` → [`spawn_editor_shell`](crate::regions::spawn_editor_shell) (the
///   four empty themed regions) then
///   [`spawn_right_panel_controls`](crate::right_panel::spawn_right_panel_controls) (the theme
///   dropdown + size selector, parented under the right panel), with the [`MapEditorSession`]
///   inserted FIRST so the controls seed from it.
/// - `OnExit(Editing)` → remove the [`MapEditorSession`] (the state-scoped-resource pattern,
///   bevy-traps #1).
/// - `Update` (in `Editing`) →
///   [`apply_theme_selection`](crate::right_panel::apply_theme_selection) (C2) +
///   [`apply_size_commit`](crate::right_panel::apply_size_commit) (C3).
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);

        // The editor's own dropdown / numeric-field per-type wiring (gate 4b). The numeric
        // field's register requires the one-time `register_text_field` seam to run first
        // (it initializes the handler registry `register_numeric_field` pushes into).
        register_dropdown::<LevelTheme>(app);
        register_text_field(app);
        register_numeric_field::<GridSpanInput>(app);

        app.add_systems(
            OnEnter(EditorState::Editing),
            (
                insert_session,
                spawn_editor_shell,
                spawn_right_panel_controls,
            )
                .chain()
                .run_if(resource_exists::<gdtf_ui::theme::GdtfTheme>),
        );
        app.add_systems(OnExit(EditorState::Editing), remove_session);
        app.add_systems(
            Update,
            (apply_theme_selection, apply_size_commit).run_if(in_state(EditorState::Editing)),
        );
    }
}

/// `OnEnter(Editing)`: insert the shared [`MapEditorSession`] (the state-scoped selection
/// state — bevy-traps #1), seeded to the default theme + the full `60×60×8` grid. The
/// default-floor key resolves on the first theme selection; the
/// [`apply_theme_selection`](crate::right_panel::apply_theme_selection) drive could also seed
/// it eagerly, but the dropdown's pre-selected default already matches the seed theme.
fn insert_session(mut commands: Commands) {
    commands.insert_resource(MapEditorSession::default());
}

/// `OnExit(Editing)`: remove the [`MapEditorSession`] so it never lingers past the editing
/// scene (the state-scoped-resource pattern — bevy-traps #1).
fn remove_session(mut commands: Commands) {
    commands.remove_resource::<MapEditorSession>();
}
