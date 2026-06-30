//! The GDTF **map editor** — a SEPARATE windowed binary from the game (GTW-417).
//!
//! This crate is the FOUNDATION of the GTW-404 map-editor track: an app shell that
//! launches a windowed editor with the theme + content registries loaded, and renders the
//! four empty themed layout regions later children populate (GTW-421 right panel, GTW-422
//! left palette, GTW-423 canvas). It mirrors `gdtf_app`'s shape — a [`MapEditorApp`]
//! wrapper over a Bevy `App` — but runs its OWN minimal [`EditorState`] machine and shares
//! NONE of the game's scene graph or battle sim (the housing constraint: the procgen
//! assembly + debug visualizer live in the main game, not the editor).
//!
//! - [`MapEditorApp`] composes `DefaultPlugins` + `gdtf_ui::UiPlugin` + [`MapEditorPlugin`]
//!   + the env-gated QA capture affordance.
//! - [`MapEditorPlugin`] wires the [`EditorState`] machine, the slim `Load` asset pass, and
//!   the [`Editing`](EditorState::Editing) scene that spawns the four regions.
//! - The four region markers — [`RightPanelRegion`], [`LeftPaletteRegion`],
//!   [`CanvasRegion`], [`StatRegion`] — let later children and the GTW-417 test find each
//!   empty container.
//! - [`EditorCapturePlugin`] is the OFF-by-default QA hook for the AC4 screenshot.
//! - [`MapEditorSession`] is the shared theme/default-floor/grid-size/selected-tile selection
//!   state the GTW-421 right-panel controls + the GTW-422 left palette write and later canvas
//!   children read. Swept onto the UUID model (GTW-495): the theme is a `ThemeUuid` and the
//!   default-floor / paint tile are `TerrainUuid`s. The `right_panel` module spawns the theme
//!   dropdown (over `UuidThemeRegistry` display names) + size selector and the `palette` module
//!   spawns the left tile palette + the bottom-right stat region.
//! - The GTW-422 `palette` module lists every terrain of the active theme's palette (sprite +
//!   name, resolved from `UuidThemeRegistry` + `TerrainDefRegistry`) in the [`LeftPaletteRegion`],
//!   writes the clicked `TerrainUuid` into the session, and shows its `TerrainDef` stats in the
//!   [`StatRegion`]; [`PaletteRow`] / [`StatText`] are its markers.
//! - The GTW-423 `canvas` module fills the [`CanvasRegion`] with the drawable cell grid — a
//!   dashed boundary + per-cell dimmed dashes around `width × height` cells each pre-filled with
//!   the theme's default-floor sprite, live-rebuilt on a theme / size change. [`CanvasRoot`] /
//!   [`CanvasCell`] / [`CanvasScroll`] / [`CanvasExtent`] are its markers. GTW-426 makes the canvas
//!   INTERACTIVE: a translucent [`CanvasGhost`] preview of the selected tile snaps to the hovered
//!   cell, and clicking a cell PAINTS it — writing the [`EditorMap`] model and redrawing the cell's
//!   sprite. GTW-500 adds the canvas UX: a [`CurrentEditLevel`] storey SELECTOR (keyboard + chrome
//!   up/down buttons + a level readout) so the canvas navigates up/down storeys (the render / paint
//!   / ghost all read it); centring of the grid in the edit viewport when it fits; and
//!   mouse-wheel ZOOM ([`CanvasZoom`], cursor-anchored, cell-size re-layout).
//! - The GTW-426 `editor_map` module owns [`EditorMap`] — the in-memory, state-scoped paintable
//!   map model (a sparse `CellLevel → TerrainUuid` store of painted cells, level-aware since
//!   GTW-430, UUID-keyed since GTW-495). It is the authoritative record the click-to-paint flow
//!   writes and the FOUNDATION the save path (GTW-432) reads.
//! - The GTW-430 `placement` module owns the SINGLE SHARED placement-legality predicate
//!   ([`evaluate_placement`]) both the hover-ghost preview and the click-commit run, plus the
//!   vertical auto-handling for multi-level tiles: placing a ladder auto-clears a slab directly
//!   above it (C1), and a slab over an existing ladder is rejected (C2). An illegal placement
//!   tints the target cell partial-transparent RED in the ghost preview and is rejected by the
//!   commit ([`PlacementVerdict`] / [`ProposedPlacement`]).
//! - The GTW-432 `save` module (debug-only) projects the [`EditorMap`] into the v2
//!   `PrefabSpecV2` schema and WRITES it to `assets/maps/<theme>/<size>/<name>.prefab_v2.ron`, so
//!   a saved prefab round-trips through the GTW-489 v2 folder loader (swept onto the v2 schema in
//!   GTW-495 — one `placements` list of `TerrainUuid` references, no per-prefab boundary
//!   openings). A prefab-name
//!   text field + a "Save prefab" button under the right panel are the live trigger; the save
//!   re-checks every painted cell through [`evaluate_placement`] so a saved prefab never contains
//!   an illegal cell.
//! - The GTW-495 `terrain_graphics` module resolves a `TerrainDef`'s `presenter_kind.graphic_name`
//!   to a terrain atlas index THE WAY THE PRESENTER DOES (through the presenter's `TileRoles`
//!   table), so the palette / canvas sprites match the battlescape's.

mod app;
mod canvas;
mod capture;
mod editor_map;
mod editor_resources;
mod load;
// GTW-474: the Workbench mode machine (the EditorMode resource + the top-bar tabs + the
// per-mode content-subtree toggle) and the status-bar refresh.
mod mode;
mod mode_host;
mod mode_status;
mod palette;
mod placement;
mod plugin;
mod regions;
mod right_panel;
// The GTW-432 save-prefab path is debug-only (the GTW-429 gang-save precedent): the whole module
// — the EditorMap → PrefabSpecV2 projection, the RON serialize, the fs-write, and the press trigger
// — is gated `#[cfg(debug_assertions)]` so it never compiles into a release binary.
#[cfg(debug_assertions)]
mod save;
mod session;
mod state;
// GTW-474: the TERRAIN authoring mode of the Workbench — the form that captures a TerrainDef and
// saves it to a per-theme `.terrain_def.ron` the GTW-487 loader resolves.
mod terrain_form;
// GTW-475: the THEME authoring mode of the Workbench — the form that assembles a UuidThemeDef BY
// REFERENCE (terrain UUIDs + a default floor) and saves it to a per-theme `.terrain_theme.ron`
// the GTW-487 theme loader resolves.
mod theme_form;
// GTW-495: resolve a TerrainDef's presenter_kind.graphic_name to a terrain atlas index THE WAY
// THE PRESENTER DOES (via the presenter's TileRoles table) — shared by the palette + canvas.
mod terrain_graphics;
mod tile_atlas;

pub use app::MapEditorApp;
pub use canvas::{
    CanvasCell, CanvasExtent, CanvasGhost, CanvasRoot, CanvasScroll, CanvasZoom, CurrentEditLevel,
    LevelNavButton, LevelReadout, ZoomReadout,
};
pub use capture::EditorCapturePlugin;
pub use editor_map::EditorMap;
pub use mode::{
    EditorMode, EditorModeTabs, PrefabModeContent, TerrainModeContent, ThemeModeContent,
};
pub use palette::{PaletteRow, StatText};
pub use placement::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement, names_a_ladder,
};
pub use plugin::MapEditorPlugin;
pub use regions::{
    CanvasRegion, EditorShellRoot, EditorStatusBar, EditorTopBar, LeftPaletteRegion,
    RightPanelRegion, StatRegion, StatusText,
};
pub use right_panel::{GridSpanInput, SizeFieldAxis, ThemeDropdown};
pub use session::MapEditorSession;
pub use state::EditorState;
pub use terrain_form::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainFootfallPicker,
    TerrainGraphicChoice, TerrainKindChoice, TerrainKindTabs, draft_to_terrain_def,
    serialize_terrain_def,
};
pub use theme_form::{
    SaveThemeError, ThemeDefaultFloorPicker, ThemeDraft, ThemeResolvedStatsText, ThemeTerrainRow,
    draft_to_theme_def, serialize_theme_def, validate_for_save,
};
