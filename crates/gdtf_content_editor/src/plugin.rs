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
use gdtf_battle_sim::level::ThemeUuid;
use gdtf_ui::{register_dropdown, register_numeric_field, register_text_field};

use crate::{
    EditorState,
    canvas::{follow_hover_ghost, paint_cell, spawn_canvas_scroll, spawn_hover_ghost, sync_canvas},
    editor_map::EditorMap,
    load::register_load,
    palette::{refresh_stat_region, select_palette_tile, spawn_stat_text, sync_palette},
    regions::spawn_editor_shell,
    right_panel::{
        GridSpanInput, apply_size_commit, apply_theme_selection, seed_default_theme,
        spawn_right_panel_controls,
    },
    session::MapEditorSession,
    tile_atlas::load_tile_atlas,
};

/// The map editor's single plugin: state machine + `Load` pass + `Editing` scene + right
/// panel controls.
///
/// Added by [`MapEditorApp`](crate::MapEditorApp) (and by the headless test) onto an app
/// that already has `DefaultPlugins` + the `gdtf_ui` `UiPlugin`. It owns:
///
/// - `init_state::<EditorState>()` — the editor's own two-state lifecycle.
/// - the slim `Load` pass (theme + weapon/armor + the UUID-keyed terrain/theme registries +
///   the presenter tile-role table) — mirrors the game's `resolve_*` loaders WITHOUT pulling the
///   game scene graph.
/// - the GTW-410 dropdown + GTW-411 numeric-field widget wiring for the editor's own option /
///   value types: `register_dropdown::<ThemeUuid>` (the theme dropdown's option id), plus
///   [`register_text_field`] (the one-time text-field seam [`register_numeric_field`] requires)
///   and `register_numeric_field::<GridSpanInput>` for the three size fields (gate 4b — no
///   unwired per-type system).
/// - `OnEnter(Editing)` → [`spawn_editor_shell`](crate::regions::spawn_editor_shell) (the
///   four empty themed regions) then
///   [`spawn_right_panel_controls`](crate::right_panel::spawn_right_panel_controls) (the theme
///   dropdown + size selector, parented under the right panel), with the [`MapEditorSession`]
///   inserted FIRST so the controls seed from it.
/// - `OnExit(Editing)` → remove the [`MapEditorSession`] (the state-scoped-resource pattern,
///   bevy-traps #1).
/// - `Update` (in `Editing`) →
///   [`apply_theme_selection`](crate::right_panel::apply_theme_selection) (GTW-421 C2) +
///   [`apply_size_commit`](crate::right_panel::apply_size_commit) (GTW-421 C3), then the
///   GTW-422 left-palette trio:
///   [`select_palette_tile`](crate::palette::select_palette_tile) (C2 — a row click sets the
///   active paint tile + highlight), [`sync_palette`](crate::palette::sync_palette) (C1 + C4 —
///   populate / repopulate the rows from the active theme's catalog), and
///   [`refresh_stat_region`](crate::palette::refresh_stat_region) (C3 — the bottom-right tile
///   stats). The GTW-422 `OnEnter` adds [`load_tile_atlas`](crate::tile_atlas::load_tile_atlas)
///   (the terrain sheet the rows draw from) and [`spawn_stat_text`](crate::palette::spawn_stat_text)
///   (the stat region's text node). The GTW-423 central canvas adds the
///   [`spawn_canvas_scroll`](crate::canvas::spawn_canvas_scroll) `OnEnter` (wraps the
///   [`CanvasRegion`](crate::CanvasRegion) in a scroll list) and the
///   [`sync_canvas`](crate::canvas::sync_canvas) `Update` (the dashed boundary, per-cell dashes,
///   and default-floor cell fill, rebuilt on a theme / size change — C1-C4), running after a
///   theme/size change has already folded into the session. The GTW-426 canvas interactivity then
///   runs after `sync_canvas`: [`paint_cell`](crate::canvas::paint_cell) (a cell click paints the
///   selected tile into the [`EditorMap`] + redraws the cell — C2/C3),
///   [`spawn_hover_ghost`](crate::canvas::spawn_hover_ghost) (the one persistent ghost overlay),
///   and [`follow_hover_ghost`](crate::canvas::follow_hover_ghost) (the ghost snaps to the hovered
///   cell — C1). `OnEnter` inserts the [`EditorMap`] paintable model; `OnExit` removes it (the
///   state-scoped-resource pattern).
pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditorState>();
        register_load(app);

        // The editor's own dropdown / numeric-field per-type wiring (gate 4b). The numeric
        // field's register requires the one-time `register_text_field` seam to run first
        // (it initializes the handler registry `register_numeric_field` pushes into).
        register_dropdown::<ThemeUuid>(app);
        register_text_field(app);
        register_numeric_field::<GridSpanInput>(app);

        app.add_systems(
            OnEnter(EditorState::Editing),
            (
                insert_session,
                insert_map,
                load_tile_atlas,
                spawn_editor_shell,
                spawn_right_panel_controls,
                spawn_stat_text,
                spawn_canvas_scroll,
            )
                .chain()
                .run_if(resource_exists::<gdtf_ui::theme::GdtfTheme>),
        );
        app.add_systems(OnExit(EditorState::Editing), (remove_session, remove_map));

        // GTW-432: the debug-only save-prefab controls (the prefab-name text field + the "Save
        // prefab" button, under the right panel) and the press trigger. Registered in their OWN
        // `#[cfg(debug_assertions)]` block so the always-compiled Update tuple below stays
        // release-buildable (the GTW-429 gang-save precedent). The spawn runs in the same gated,
        // theme-guarded `OnEnter` slot as the other right-panel controls.
        #[cfg(debug_assertions)]
        {
            use crate::save::{save_prefab_on_press, spawn_save_controls};

            app.add_systems(
                OnEnter(EditorState::Editing),
                spawn_save_controls.run_if(resource_exists::<gdtf_ui::theme::GdtfTheme>),
            );
            app.add_systems(
                Update,
                save_prefab_on_press.run_if(in_state(EditorState::Editing)),
            );
        }

        app.add_systems(
            Update,
            (
                // Seed the session theme to the dropdown's pre-selected default once the
                // UuidThemeRegistry resolves (the session opens on the nil theme), BEFORE the
                // theme-driven systems read it (C1).
                seed_default_theme,
                apply_theme_selection,
                apply_size_commit,
                // The palette sync + select run AFTER the theme dropdown folds its selection into
                // the session, so a theme switch this frame repopulates the palette (C4); a row
                // click updates the selection + highlight (C2). The stat refresh runs LAST so it
                // observes the selection write this frame (C3).
                select_palette_tile,
                sync_palette,
                refresh_stat_region,
                // The canvas syncs after a theme switch / size commit has folded into the session
                // (apply_theme_selection / apply_size_commit) so the grid re-fills / re-extents
                // (C4). The GTW-426 interactivity then runs AFTER the canvas exists this frame:
                // `paint_cell` reads a cell press (C2/C3), `spawn_hover_ghost` ensures the one ghost
                // exists, and `follow_hover_ghost` snaps it to the hovered cell (C1).
                sync_canvas,
                paint_cell,
                spawn_hover_ghost,
                follow_hover_ghost,
            )
                .chain()
                .run_if(in_state(EditorState::Editing)),
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

/// `OnEnter(Editing)`: insert the empty [`EditorMap`] paintable model (the state-scoped
/// click-to-paint store — bevy-traps #1, GTW-426). Starts empty (nothing painted; every cell
/// renders the theme default-floor); the click-to-paint flow ([`paint_cell`]) writes it.
fn insert_map(mut commands: Commands) {
    commands.insert_resource(EditorMap::new());
}

/// `OnExit(Editing)`: remove the [`EditorMap`] so the painted map never lingers past the editing
/// scene (the state-scoped-resource pattern — bevy-traps #1).
fn remove_map(mut commands: Commands) {
    commands.remove_resource::<EditorMap>();
}
