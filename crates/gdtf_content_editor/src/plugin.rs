//! The [`MapEditorPlugin`] — the editor's single registration seam.
//!
//! Wires the editor's own [`EditorState`] machine, its slim `Load` asset pass
//! ([`register_load`](crate::load::register_load)), the [`Editing`](EditorState::Editing) scene's
//! state-scoped model/resource lifecycle ([`editor_resources`](crate::editor_resources)), the
//! standalone editor camera ([`spawn_editor_camera`](crate::camera::spawn_editor_camera)), and —
//! since the GTW-512 egui swap — the single egui Workbench-shell UI system
//! ([`editor_egui_ui`](crate::egui_shell::editor_egui_ui)) in the
//! [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) schedule.
//!
//! It does NOT register any of the game's scene plugins or the battle sim runtime (the GTW-417
//! housing constraint).
//!
//! ## GTW-512: the clean swap off `bevy_ui`
//!
//! The hand-rolled `bevy_ui` shell — `spawn_editor_shell`, the four-region row, the mode-tab
//! segmented control, the theme dropdown / numeric-field widgets, the palette / canvas / right-panel
//! drive systems, and the per-mode `Visibility`-container toggle — is GONE. egui draws the WHOLE
//! shell in ONE system in `EguiPrimaryContextPass`; the kept model resources (the
//! [`EditorMode`](crate::mode::EditorMode), the [`MapEditorSession`](crate::session::MapEditorSession),
//! the [`EditorMap`](crate::editor_map::EditorMap), the level / zoom selectors, the terrain / theme
//! drafts) are still inserted on enter + removed on exit, plus the new
//! [`HoveredCell`](crate::hovered_cell::HoveredCell) model resource (C1.5) the live egui hover and
//! the QA capture both write. The full per-mode FORMS are stubbed in C1 (the egui shell) — the
//! TERRAIN / THEME / PREFAB forms + the texture viewport are the later children C2 / C3 / C4.

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::{
    EditorState,
    camera::{disable_egui_auto_context, spawn_editor_camera},
    editor_resources::{
        insert_canvas_zoom, insert_edit_level, insert_hovered_cell, insert_map, insert_mode,
        insert_preview_pan, insert_session, insert_terrain_draft, insert_theme_draft,
        remove_canvas_zoom, remove_edit_level, remove_hovered_cell, remove_map, remove_mode,
        remove_preview_pan, remove_session, remove_terrain_draft, remove_theme_draft,
    },
    egui_shell::{editor_egui_ui, level_nav_hotkeys},
    load::register_load,
    mode::mode_hotkeys,
    preview::register_preview,
    right_panel::seed_default_theme,
    tile_atlas::load_tile_atlas,
};

/// The map editor's single plugin: state machine + `Load` pass + `Editing` scene + the egui shell.
///
/// Added by [`MapEditorApp`](crate::MapEditorApp) (and by the headless test) onto an app that
/// already has `DefaultPlugins` + the [`EguiPlugin`](bevy_egui::EguiPlugin). It owns:
///
/// - `init_state::<EditorState>()` — the editor's own two-state lifecycle.
/// - the slim `Load` pass (theme + weapon/armor + the UUID-keyed terrain/theme registries + the
///   presenter tile-role table) — mirrors the game's `resolve_*` loaders WITHOUT pulling the game
///   scene graph.
/// - `OnEnter(Editing)` → the standalone editor camera + the FULL state-scoped model/resource
///   insert chain (the Workbench mode + the authoring session + the paintable map + the level / zoom
///   selectors + the terrain / theme drafts + the new hovered-cell model + the tile atlas).
/// - `OnExit(Editing)` → the matching removes (the state-scoped-resource pattern — bevy-traps #1).
/// - `EguiPrimaryContextPass` (in `Editing`) → [`editor_egui_ui`] draws the whole shell (mode tabs,
///   global theme `ComboBox`, status line, palette/stats placeholder, the active mode's stubbed form,
///   the viewport placeholder). egui systems live in `EguiPrimaryContextPass`, NOT `Update`
///   (bevy-traps: a `Update` `ctx_mut()` call fights the egui begin/end-pass plumbing).
/// - `Update` (in `Editing`) → the UI-agnostic model drives kept from the old shell:
///   [`seed_default_theme`] (seed the session theme to the registry's first theme once it resolves)
///   and [`mode_hotkeys`] (the `1`/`2`/`3` mode hotkeys). The theme SELECTION is now folded into the
///   session by the egui `ComboBox` directly (resolve the chosen theme's default-floor +
///   [`MapEditorSession::select_theme`](crate::session::MapEditorSession::select_theme)) — the verbatim body the old gdtf_ui-coupled
///   `apply_theme_selection` ran, which is gone now its dropdown message no longer fires.
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);

        // GTW-515: disable bevy_egui's auto-attach of the primary context (the editor has TWO
        // cameras now — the window camera + the offscreen prefab preview camera — so auto-attach to
        // the ambiguously-first camera could land on the preview camera and blank the window). The
        // primary context is attached EXPLICITLY to the window camera in `spawn_editor_camera`. Must
        // run before any camera spawns (Startup, before the OnEnter(Editing) camera spawns).
        app.add_systems(Startup, disable_egui_auto_context);

        app.add_systems(
            OnEnter(EditorState::Editing),
            (
                spawn_editor_camera,
                insert_mode,
                insert_session,
                insert_map,
                insert_edit_level,
                insert_canvas_zoom,
                insert_terrain_draft,
                insert_theme_draft,
                // GTW-512 C1.5: the hovered-cell model the live egui hover + the QA capture write.
                insert_hovered_cell,
                // GTW-515 C4.8: the owned pan-offset target (the zoom target is the kept CanvasZoom).
                insert_preview_pan,
                load_tile_atlas,
            ),
        );
        app.add_systems(
            OnExit(EditorState::Editing),
            (
                remove_mode,
                remove_session,
                remove_map,
                remove_edit_level,
                remove_canvas_zoom,
                remove_terrain_draft,
                remove_theme_draft,
                remove_hovered_cell,
                remove_preview_pan,
            ),
        );

        // GTW-515 C4.3: the prefab preview render machinery — the offscreen render-target image +
        // the dedicated isolated-render-layer camera (OnEnter/OnExit), the change-driven tile
        // redraw, and the once-per-frame set-to-target zoom/pan apply (Update, in Editing).
        register_preview(app);

        // GTW-512 C1.3: the egui shell — ONE UI system in `EguiPrimaryContextPass` (NOT `Update`),
        // gated to `Editing`. It declares the panels outermost-first with the central panel LAST
        // (egui's load-bearing panel order).
        app.add_systems(
            EguiPrimaryContextPass,
            editor_egui_ui.run_if(in_state(EditorState::Editing)),
        );

        // The UI-agnostic model drives kept from the old shell (no `bevy_ui` dependency): the theme
        // seed + the mode hotkeys. The theme SELECTION is folded into the session by the egui
        // `ComboBox` directly (the verbatim `apply_theme_selection` body), so that `gdtf_ui`-coupled
        // drive is not wired.
        app.add_systems(
            Update,
            (seed_default_theme, mode_hotkeys, level_nav_hotkeys)
                .run_if(in_state(EditorState::Editing)),
        );
    }
}
