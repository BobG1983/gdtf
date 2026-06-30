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
use gdtf_battle_sim::{level::ThemeUuid, terrain::def::TerrainUuid};
use gdtf_ui::{register_dropdown, register_numeric_field, register_text_field};

use crate::{
    EditorState,
    canvas::{follow_hover_ghost, paint_cell, spawn_canvas_scroll, spawn_hover_ghost, sync_canvas},
    editor_map::EditorMap,
    load::register_load,
    mode::{EditorMode, apply_mode_switch, mode_hotkeys, sync_tabs_to_mode, toggle_mode_content},
    mode_status::refresh_status_bar,
    palette::{refresh_stat_region, select_palette_tile, spawn_stat_text, sync_palette},
    regions::spawn_editor_shell,
    right_panel::{
        GridSpanInput, apply_size_commit, apply_theme_selection, seed_default_theme,
        spawn_right_panel_controls,
    },
    session::MapEditorSession,
    terrain_form::{
        ArmorInput, FootfallChoice, HpInput, TerrainDraft, apply_terrain_band,
        apply_terrain_footfall, apply_terrain_kind, commit_terrain_armor, commit_terrain_hp,
        commit_terrain_name, gate_footfall_field, reflow_band_field, refresh_ron_preview,
        select_terrain_graphic, spawn_terrain_form, toggle_terrain_tag,
    },
    theme_form::{
        ThemeDraft, apply_default_floor, commit_theme_name, load_theme_into_form,
        refresh_resolved_stats, refresh_theme_key_text, refresh_theme_ron_preview,
        reset_theme_form_on_new, spawn_theme_form, sync_default_floor_options, sync_theme_library,
        toggle_theme_terrain,
    },
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
        register_text_field(app);
        register_dropdown::<ThemeUuid>(app);
        register_numeric_field::<GridSpanInput>(app);
        // GTW-474: the TERRAIN form's own dropdown / numeric-field per-type wiring — the footfall
        // dropdown and the HP / armor numeric fields.
        register_dropdown::<FootfallChoice>(app);
        register_numeric_field::<HpInput>(app);
        register_numeric_field::<ArmorInput>(app);
        // GTW-475: the THEME form's own dropdown wiring — the default-floor dropdown is over the
        // sim's TerrainUuid (the theme references terrain by UUID).
        register_dropdown::<TerrainUuid>(app);

        app.add_systems(
            OnEnter(EditorState::Editing),
            (
                // GTW-474/475: the Workbench mode resource + the TERRAIN + THEME drafts inserted
                // FIRST so the mode toggle + the forms seed from them (state-scoped-resource).
                insert_mode,
                insert_session,
                insert_map,
                insert_terrain_draft,
                insert_theme_draft,
                load_tile_atlas,
                // The shell spawns the four regions + the per-mode content containers + the top
                // bar (mode tabs + the promoted theme dropdown) + the status bar.
                spawn_editor_shell,
                // PREFAB-mode content (parents into the regions' PrefabModeContent containers).
                spawn_right_panel_controls,
                spawn_stat_text,
                spawn_canvas_scroll,
                // TERRAIN-mode content (parents into the regions' TerrainModeContent containers).
                spawn_terrain_form,
                // THEME-mode content (parents into the regions' ThemeModeContent containers).
                spawn_theme_form,
            )
                .chain()
                .run_if(resource_exists::<gdtf_ui::theme::GdtfTheme>),
        );
        app.add_systems(
            OnExit(EditorState::Editing),
            (
                remove_mode,
                remove_session,
                remove_map,
                remove_terrain_draft,
                remove_theme_draft,
            ),
        );

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

        register_workbench_systems(app);
    }
}

/// Registers the GTW-474 Workbench `Update` systems: the MODE machine + the TERRAIN form drive
/// (+ the debug-only terrain save). Factored out of [`MapEditorPlugin::build`] so it stays under
/// the clippy line gate (the wiring is mechanical; the ordering rationale is at each block).
fn register_workbench_systems(app: &mut App) {
    // The Workbench MODE machine. `apply_mode_switch` (a tab click) + `mode_hotkeys` (the 1/2
    // keys) both write `EditorMode`; `toggle_mode_content` then flips the per-mode containers'
    // Visibility, `sync_tabs_to_mode` follows the tabs to a hotkey-driven change, and
    // `refresh_status_bar` rewrites the status line — all AFTER the mode is written this frame
    // (C1).
    app.add_systems(
        Update,
        (
            apply_mode_switch,
            mode_hotkeys,
            toggle_mode_content,
            sync_tabs_to_mode,
            refresh_status_bar,
        )
            .chain()
            .run_if(in_state(EditorState::Editing)),
    );

    // The TERRAIN form's drive systems. The commits (name / kind / band / HP / armor / graphic /
    // footfall / tags) run first, then the kind-driven reflow (the C2 footfall gate + the band
    // field), then the live RON preview LAST so it reflects this frame's edits (C2).
    app.add_systems(
        Update,
        (
            commit_terrain_name,
            apply_terrain_kind,
            apply_terrain_band,
            commit_terrain_hp,
            commit_terrain_armor,
            select_terrain_graphic,
            apply_terrain_footfall,
            toggle_terrain_tag,
            gate_footfall_field,
            reflow_band_field,
            refresh_ron_preview,
        )
            .chain()
            .run_if(in_state(EditorState::Editing)),
    );

    // The debug-only terrain SAVE trigger (the fs-write press) — its own `#[cfg(debug_assertions)]`
    // block so the always-compiled tuples above stay release-buildable (the GTW-432 prefab-save
    // precedent).
    #[cfg(debug_assertions)]
    {
        use crate::terrain_form::save_terrain_on_press;

        app.add_systems(
            Update,
            save_terrain_on_press.run_if(in_state(EditorState::Editing)),
        );
    }

    // GTW-475: the THEME form's drive systems. A top-bar dropdown change LOADS a theme (C4) +
    // the New-theme press RESETS the form (C4) run FIRST (they replace the draft), then the name
    // commit + the terrain multi-select toggle, then the library / default-floor option sync
    // (which depend on the draft + the registry), the default-floor pick, the key text, and the
    // resolved-stats + RON preview LAST so they reflect this frame's edits (C2/C3).
    app.add_systems(
        Update,
        (
            load_theme_into_form,
            reset_theme_form_on_new,
            commit_theme_name,
            toggle_theme_terrain,
            sync_theme_library,
            sync_default_floor_options,
            apply_default_floor,
            refresh_theme_key_text,
            refresh_resolved_stats,
            refresh_theme_ron_preview,
        )
            .chain()
            .run_if(in_state(EditorState::Editing)),
    );

    // The debug-only theme SAVE trigger (the fs-write press) — its own `#[cfg(debug_assertions)]`
    // block so the always-compiled tuple above stays release-buildable (the GTW-474 precedent).
    #[cfg(debug_assertions)]
    {
        use crate::theme_form::save_theme_on_press;

        app.add_systems(
            Update,
            save_theme_on_press.run_if(in_state(EditorState::Editing)),
        );
    }
}

/// `OnEnter(Editing)`: insert the [`EditorMode`] resource (the state-scoped Workbench mode —
/// bevy-traps #1), seeded to the default [`Prefab`](EditorMode::Prefab) mode so the editor opens
/// in the existing painter (GTW-474).
fn insert_mode(mut commands: Commands) {
    commands.insert_resource(EditorMode::default());
}

/// `OnExit(Editing)`: remove the [`EditorMode`] resource (the state-scoped-resource pattern —
/// bevy-traps #1).
fn remove_mode(mut commands: Commands) {
    commands.remove_resource::<EditorMode>();
}

/// `OnEnter(Editing)`: insert the [`TerrainDraft`] (the state-scoped TERRAIN-mode authoring draft
/// — bevy-traps #1), seeded to a fresh default the form's controls seed their initial values from
/// (GTW-474).
fn insert_terrain_draft(mut commands: Commands) {
    commands.insert_resource(TerrainDraft::default());
}

/// `OnExit(Editing)`: remove the [`TerrainDraft`] (the state-scoped-resource pattern —
/// bevy-traps #1).
fn remove_terrain_draft(mut commands: Commands) {
    commands.remove_resource::<TerrainDraft>();
}

/// `OnEnter(Editing)`: insert the [`ThemeDraft`] (the state-scoped THEME-mode authoring draft —
/// bevy-traps #1), seeded to a fresh NEW-theme draft (a minted key, an empty form — C4) the
/// form's controls seed their initial values from (GTW-475).
fn insert_theme_draft(mut commands: Commands) {
    commands.insert_resource(ThemeDraft::default());
}

/// `OnExit(Editing)`: remove the [`ThemeDraft`] (the state-scoped-resource pattern —
/// bevy-traps #1).
fn remove_theme_draft(mut commands: Commands) {
    commands.remove_resource::<ThemeDraft>();
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
