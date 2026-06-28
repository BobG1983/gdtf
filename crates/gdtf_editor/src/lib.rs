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
//!   children read; the `right_panel` module spawns the theme dropdown + size selector and the
//!   `palette` module spawns the left tile palette + the bottom-right stat region.
//! - The GTW-422 `palette` module lists every tile of the active theme (sprite + name) in the
//!   [`LeftPaletteRegion`], writes the clicked tile into the session, and shows its catalog
//!   stats in the [`StatRegion`]; [`PaletteRow`] / [`StatText`] are its markers.
//! - The GTW-423 `canvas` module fills the [`CanvasRegion`] with the drawable cell grid — a
//!   dashed boundary + per-cell dimmed dashes around `width × height` cells each pre-filled with
//!   the theme's default-floor sprite, live-rebuilt on a theme / size change (2D x/y plane only;
//!   z is out of scope). [`CanvasRoot`] / [`CanvasCell`] / [`CanvasScroll`] / [`CanvasExtent`]
//!   are its markers. GTW-426 makes the canvas INTERACTIVE: a translucent [`CanvasGhost`] preview
//!   of the selected tile snaps to the hovered cell, and clicking a cell PAINTS it — writing the
//!   [`EditorMap`] model and redrawing the cell's sprite.
//! - The GTW-426 `editor_map` module owns [`EditorMap`] — the in-memory, state-scoped paintable
//!   map model (a sparse `CellLevel → TileKey` store of painted cells, level-aware since GTW-430).
//!   It is the authoritative record the click-to-paint flow writes and the FOUNDATION the
//!   save/emit tickets (GTW-429 / GTW-431 / GTW-432) will read.
//! - The GTW-430 `placement` module owns the SINGLE SHARED placement-legality predicate
//!   ([`evaluate_placement`]) both the hover-ghost preview and the click-commit run, plus the
//!   vertical auto-handling for multi-level tiles: placing a ladder auto-clears a slab directly
//!   above it (C1), and a slab over an existing ladder is rejected (C2). An illegal placement
//!   tints the target cell partial-transparent RED in the ghost preview and is rejected by the
//!   commit ([`PlacementVerdict`] / [`ProposedPlacement`]).
//! - The GTW-432 `save` module (debug-only) projects the [`EditorMap`] into the canonical GTW-418
//!   `PrefabSpec` schema and WRITES it to `assets/content/maps/<theme>/<size>/<name>.prefab.ron`,
//!   so a saved prefab round-trips through the GTW-418 folder loader. A prefab-name text field +
//!   a "Save prefab" button under the right panel are the live trigger; the save re-checks every
//!   painted cell through [`evaluate_placement`] so a saved prefab never contains an illegal cell.

mod app;
mod canvas;
mod capture;
mod editor_map;
mod load;
mod palette;
mod placement;
mod plugin;
mod regions;
mod right_panel;
// The GTW-432 save-prefab path is debug-only (the GTW-429 gang-save precedent): the whole module
// — the EditorMap → PrefabSpec projection, the RON serialize, the fs-write, and the press trigger
// — is gated `#[cfg(debug_assertions)]` so it never compiles into a release binary.
#[cfg(debug_assertions)]
mod save;
mod session;
mod state;
mod tile_atlas;

pub use app::MapEditorApp;
pub use canvas::{CanvasCell, CanvasExtent, CanvasGhost, CanvasRoot, CanvasScroll};
pub use capture::EditorCapturePlugin;
pub use editor_map::EditorMap;
pub use palette::{PaletteRow, StatText};
pub use placement::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement, names_a_ladder,
};
pub use plugin::MapEditorPlugin;
pub use regions::{CanvasRegion, EditorShellRoot, LeftPaletteRegion, RightPanelRegion, StatRegion};
pub use right_panel::{GridSpanInput, SizeFieldAxis, ThemeDropdown};
pub use session::MapEditorSession;
pub use state::EditorState;
