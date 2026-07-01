//! The editor's state-scoped RESOURCE lifecycle systems — the `OnEnter(Editing)` inserts and
//! `OnExit(Editing)` removes for every editor-scoped resource (bevy-traps #1: Bevy has no built-in
//! state-scoped resource, so each is inserted on enter and removed on exit by a tiny system).
//!
//! Factored out of [`plugin`](crate::plugin) so the plugin file stays a focused registration seam
//! under the block size cap — the lifecycle systems are mechanical one-liners with a single shared
//! concern (the state-scoped-resource pattern), so they live together here.

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;

use crate::{
    canvas::{CanvasZoom, CurrentEditLevel},
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    mode::EditorMode,
    preview::view::PreviewPan,
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
};

/// `OnEnter(Editing)`: insert the [`EditorMode`] resource (the state-scoped Workbench mode —
/// bevy-traps #1), seeded to the default [`Prefab`](EditorMode::Prefab) mode so the editor opens
/// in the existing painter (GTW-474).
pub(crate) fn insert_mode(mut commands: Commands) {
    commands.insert_resource(EditorMode::default());
}

/// `OnExit(Editing)`: remove the [`EditorMode`] resource (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_mode(mut commands: Commands) {
    commands.remove_resource::<EditorMode>();
}

/// `OnEnter(Editing)`: insert the [`TerrainDraft`] (the state-scoped TERRAIN-mode authoring draft
/// — bevy-traps #1), seeded to a fresh default the form's controls seed their initial values from
/// (GTW-474).
pub(crate) fn insert_terrain_draft(mut commands: Commands) {
    commands.insert_resource(TerrainDraft::default());
}

/// `OnExit(Editing)`: remove the [`TerrainDraft`] (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_terrain_draft(mut commands: Commands) {
    commands.remove_resource::<TerrainDraft>();
}

/// `OnEnter(Editing)`: insert the [`ThemeDraft`] (the state-scoped THEME-mode authoring draft —
/// bevy-traps #1), seeded to a fresh NEW-theme draft (a minted key, an empty form — C4) the
/// form's controls seed their initial values from (GTW-475).
pub(crate) fn insert_theme_draft(mut commands: Commands) {
    commands.insert_resource(ThemeDraft::default());
}

/// `OnExit(Editing)`: remove the [`ThemeDraft`] (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_theme_draft(mut commands: Commands) {
    commands.remove_resource::<ThemeDraft>();
}

/// `OnEnter(Editing)`: insert the shared [`MapEditorSession`] (the state-scoped selection
/// state — bevy-traps #1), seeded to the default theme + the full `60×60×8` grid. The
/// default-floor key resolves on the first theme selection;
/// [`seed_default_theme`](crate::right_panel::seed_default_theme) seeds it eagerly once the registry
/// resolves, and the egui theme `ComboBox` re-resolves it on a pick.
pub(crate) fn insert_session(mut commands: Commands) {
    commands.insert_resource(MapEditorSession::default());
}

/// `OnExit(Editing)`: remove the [`MapEditorSession`] so it never lingers past the editing
/// scene (the state-scoped-resource pattern — bevy-traps #1).
pub(crate) fn remove_session(mut commands: Commands) {
    commands.remove_resource::<MapEditorSession>();
}

/// `OnEnter(Editing)`: insert the empty [`EditorMap`] paintable model (the state-scoped
/// click-to-paint store — bevy-traps #1, GTW-426). Starts empty (nothing painted; every cell
/// renders the theme default-floor); the click-to-paint flow writes it.
pub(crate) fn insert_map(mut commands: Commands) {
    commands.insert_resource(EditorMap::new());
}

/// `OnExit(Editing)`: remove the [`EditorMap`] so the painted map never lingers past the editing
/// scene (the state-scoped-resource pattern — bevy-traps #1).
pub(crate) fn remove_map(mut commands: Commands) {
    commands.remove_resource::<EditorMap>();
}

/// `OnEnter(Editing)`: insert the [`CurrentEditLevel`] selector (state-scoped — bevy-traps #1,
/// GTW-500 C1), seeded to the ground storey so the editor opens on the same plane the GTW-423
/// canvas drew. The level-nav systems step it; the canvas render / paint / ghost read it.
pub(crate) fn insert_edit_level(mut commands: Commands) {
    commands.insert_resource(CurrentEditLevel::ground());
}

/// `OnExit(Editing)`: remove the [`CurrentEditLevel`] selector (the state-scoped-resource pattern
/// — bevy-traps #1).
pub(crate) fn remove_edit_level(mut commands: Commands) {
    commands.remove_resource::<CurrentEditLevel>();
}

/// `OnEnter(Editing)`: insert the [`CanvasZoom`] factor (state-scoped — bevy-traps #1, GTW-500
/// C3), seeded to the unzoomed `1.0` so the editor opens at the GTW-423 base cell scale. The
/// wheel / reset systems write it; `apply_canvas_zoom` re-lays-out the cells from it.
pub(crate) fn insert_canvas_zoom(mut commands: Commands) {
    commands.insert_resource(CanvasZoom::identity());
}

/// `OnExit(Editing)`: remove the [`CanvasZoom`] factor (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_canvas_zoom(mut commands: Commands) {
    commands.remove_resource::<CanvasZoom>();
}

/// `OnEnter(Editing)`: insert the [`PreviewPan`] offset (state-scoped — bevy-traps #1, GTW-515
/// C4.8), seeded to the origin so the preview opens centred on the world origin. The viewport
/// right-drag folds into it (set-to-target); `apply_preview_view` drives the camera from it. Its
/// sibling zoom target is the kept [`CanvasZoom`] (inserted by [`insert_canvas_zoom`]).
pub(crate) fn insert_preview_pan(mut commands: Commands) {
    commands.insert_resource(PreviewPan::origin());
}

/// `OnExit(Editing)`: remove the [`PreviewPan`] offset (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_preview_pan(mut commands: Commands) {
    commands.remove_resource::<PreviewPan>();
}

/// `OnEnter(Editing)`: insert the [`HoveredCell`] model (state-scoped — bevy-traps #1, GTW-512
/// C1.5), seeded empty (nothing hovered). The live egui viewport hover (C4) and the QA capture
/// drive both write it; the preview ghost reads it.
pub(crate) fn insert_hovered_cell(mut commands: Commands) {
    commands.insert_resource(HoveredCell::new());
}

/// `OnExit(Editing)`: remove the [`HoveredCell`] model (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_hovered_cell(mut commands: Commands) {
    commands.remove_resource::<HoveredCell>();
}

/// `OnEnter(Editing)`: insert the prefab-viewport [`ViewMode`] (state-scoped — bevy-traps #1,
/// GTW-532), seeded to the DEFAULT [`ViewMode::DownToActive`] (draw `0..=CurrentEditLevel`) so the
/// preview opens with the GTW-515 down-to-active behaviour. REUSES the presenter's [`ViewMode`]
/// TYPE verbatim (the SAME resource the battlescape's GTW-521 full-view toggle drives) — the prefab
/// viewport reads it for its drawn storey upper-bound, and the prefab full-view toggle flips it.
pub(crate) fn insert_view_mode(mut commands: Commands) {
    commands.insert_resource(ViewMode::default());
}

/// `OnExit(Editing)`: remove the prefab-viewport [`ViewMode`] (the state-scoped-resource pattern —
/// bevy-traps #1).
pub(crate) fn remove_view_mode(mut commands: Commands) {
    commands.remove_resource::<ViewMode>();
}
