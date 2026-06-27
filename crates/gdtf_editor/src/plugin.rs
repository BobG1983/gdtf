//! The [`MapEditorPlugin`] — the editor's single registration seam.
//!
//! Wires the editor's own [`EditorState`] machine, its slim `Load` asset pass
//! ([`register_load`](crate::load::register_load)), and the [`Editing`](EditorState::Editing)
//! scene that spawns the four empty themed regions
//! ([`spawn_editor_shell`](crate::regions::spawn_editor_shell)). It does NOT register any of
//! the game's scene plugins or the battle sim runtime (the GTW-417 housing constraint).

use bevy::prelude::*;

use crate::{EditorState, load::register_load, regions::spawn_editor_shell};

/// The map editor's single plugin: state machine + `Load` pass + `Editing` scene.
///
/// Added by [`MapEditorApp`](crate::MapEditorApp) (and by the headless test) onto an app
/// that already has `DefaultPlugins` + the `gdtf_ui` `UiPlugin`. It owns:
///
/// - `init_state::<EditorState>()` — the editor's own two-state lifecycle.
/// - the slim `Load` pass (theme + weapon/armor/theme-catalog registries) — mirrors the
///   game's `resolve_*` loaders WITHOUT pulling the game scene graph.
/// - `OnEnter(Editing)` → [`spawn_editor_shell`] — the four empty themed regions.
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);
        app.add_systems(
            OnEnter(EditorState::Editing),
            spawn_editor_shell.run_if(resource_exists::<gdtf_ui::theme::GdtfTheme>),
        );
    }
}
