//! The GDTF **map editor** — a SEPARATE windowed binary from the game (GTW-417).
//!
//! This crate is the FOUNDATION of the GTW-404 map-editor track: an app shell that launches a
//! windowed editor with the theme + content registries loaded. It mirrors `gdtf_app`'s shape — a
//! [`MapEditorApp`] wrapper over a Bevy `App` — but runs its OWN minimal [`EditorState`] machine and
//! shares NONE of the game's scene graph or battle sim (the housing constraint: the procgen assembly
//! + debug visualizer live in the main game, not the editor).
//!
//! ## GTW-512: the egui Workbench shell (C1 of the GTW-511 migration)
//!
//! The editor's UI is `bevy_egui` (GTW-512) — a CLEAN SWAP off the hand-rolled `gdtf_ui` shell (the
//! four `bevy_ui` regions + the segmented-control mode tabs + the dropdown/numeric-field widgets +
//! the palette / canvas / right-panel drive systems are GONE). The egui shell draws the WHOLE
//! editor in ONE system in the [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass) schedule
//! (mode tabs + global theme `ComboBox`, a status line, a palette/stats placeholder, the active mode's
//! form, and a viewport placeholder), reading the kept MODEL resources. C1 is the SHELL + the capture
//! re-point; the three per-mode FORMS are stubbed minimally (the full TERRAIN / THEME / PREFAB forms +
//! the texture viewport are the later children C2 / C3 / C4).
//!
//! - [`MapEditorApp`] composes `DefaultPlugins` + [`EguiPlugin`](bevy_egui::EguiPlugin) +
//!   [`MapEditorPlugin`] + the env-gated QA capture affordance.
//! - [`MapEditorPlugin`] wires the [`EditorState`] machine, the slim `Load` asset pass, the
//!   [`Editing`](EditorState::Editing) scene's state-scoped model lifecycle + the standalone camera,
//!   and the egui shell UI system.
//! - [`EditorCapturePlugin`] is the OFF-by-default QA hook for the screenshot (re-pointed in GTW-512
//!   to drive the MODEL resources directly + the new [`HoveredCell`], so it stays QA-able under egui).
//! - [`MapEditorSession`] is the shared theme/default-floor/grid-size/selected-tile selection state
//!   the egui shell writes and the (kept) model reads. Swept onto the UUID model (GTW-495): the theme
//!   is a `ThemeUuid` and the default-floor / paint tile are `TerrainUuid`s.
//! - [`EditorMode`] is the state-scoped active-mode resource the egui mode tabs + the `1`/`2`/`3`
//!   hotkeys ([`mode`]) drive; the egui shell branches its right panel on it.
//! - [`HoveredCell`] is the hovered-cell MODEL the live egui hover (C4) + the QA capture both write,
//!   so the preview ghost is QA-able headlessly (GTW-512 C1.5).
//! - The `canvas` module keeps the two MODEL resources the editor's lifecycle inserts —
//!   [`CurrentEditLevel`] (the storey selector — GTW-500 C1) + [`CanvasZoom`] (the viewport zoom —
//!   GTW-500 C3, reused as the preview camera's `OrthographicProjection::scale`); the egui viewport
//!   (GTW-515 C4) reads them. The `bevy_ui` cell-grid render is gone.
//! - The GTW-515 `preview` module owns the render-to-texture VIEWPORT (C4.3): an offscreen render
//!   target [`Image`](bevy::image::Image), a dedicated second [`Camera2d`](bevy::prelude::Camera2d)
//!   rendering the prefab preview tiles into it on an ISOLATED
//!   [`RenderLayers`](bevy::camera::visibility::RenderLayers), the change-driven tile redraw + hover
//!   ghost, and the once-per-frame set-to-target zoom/pan apply. The egui PREFAB mode (the
//!   `egui_shell::prefab` submodule) draws that registered image as its central-panel viewport,
//!   folding click / wheel / drag input into the [`EditorMap`] + [`CanvasZoom`] + [`PreviewPan`].
//! - The GTW-426 `editor_map` module owns [`EditorMap`] — the in-memory, state-scoped paintable map
//!   model (a sparse `CellLevel → TerrainUuid` store of painted cells, level-aware since GTW-430,
//!   UUID-keyed since GTW-495). The authoritative record the paint flow writes + the save path reads.
//! - The GTW-430 `placement` module owns the SINGLE SHARED placement-legality predicate
//!   ([`evaluate_placement`]) the preview + the commit run, plus the multi-level auto-handling
//!   ([`PlacementVerdict`] / [`ProposedPlacement`]).
//! - The GTW-432 `save` module (debug-only) keeps the pure PROJECTION of the [`EditorMap`] into the
//!   [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) schema + the RON serialize + the path resolution (the egui save controls + the
//!   fs-write press are deferred to the C4 child); the projection re-checks every painted cell through
//!   [`evaluate_placement`] so a saved prefab never contains an illegal cell.
//! - The `right_panel` module keeps the size-selector / theme-dropdown model TYPES
//!   ([`GridSpanInput`] / [`SizeFieldAxis`] / [`ThemeDropdown`]) + [`seed_default_theme`](right_panel)
//!   for the children to rebuild the controls in egui.
//! - The GTW-495 `terrain_graphics` module resolves a `TerrainDef`'s `presenter_kind.graphic_name`
//!   to a terrain atlas index THE WAY THE PRESENTER DOES (through the presenter's `TileRoles` table).

mod app;
// GTW-512 C1.2: the editor's standalone 2D camera (the `bevy_ui` shell used to spawn it).
mod camera;
mod canvas;
mod capture;
// GTW-531: the prefab-editor vertical-connector auto-pairing — placing an UP connector at (x,y,N)
// also places its paired DOWN connector at (x,y,N+1). REUSES the shared placement predicate; a
// prefab-editor placement rule ONLY (no sim/runtime change).
mod connector_pairing;
// GTW-512 C1: the egui Workbench shell — the CLEAN SWAP off the hand-rolled `bevy_ui` shell.
mod editor_map;
mod editor_resources;
mod egui_shell;
// GTW-512 C1.5: the hovered-cell model the live egui hover + the QA capture both write.
mod hovered_cell;
mod load;
// GTW-474: the Workbench mode machine (the EditorMode resource) — GTW-512 trimmed it to the enum +
// the `1`/`2`/`3` hotkeys (the egui shell draws the tabs + branches the right panel in-UI).
mod mode;
mod placement;
mod plugin;
// GTW-515 C4.3: the prefab preview RENDER MACHINERY — an offscreen render-target image, a dedicated
// second Camera2d on an isolated RenderLayers that renders the prefab preview tiles into it, and the
// change-driven tile redraw + the set-to-target zoom/pan apply. The egui PREFAB mode draws this
// registered image as its central-panel viewport.
mod preview;
// GTW-421 size-selector / theme-dropdown model TYPES + the `seed_default_theme` drive, kept across
// the egui swap (GTW-512); the `bevy_ui` spawn + the gdtf_ui-widget commit drives were dropped.
mod right_panel;
// The GTW-432 save-prefab path is debug-only (the GTW-429 gang-save precedent): the whole module
// — the EditorMap → PrefabSpec projection, the RON serialize, the fs-write, and the press trigger
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
// GTW-512: the `bevy_ui` canvas render markers are GONE (the egui viewport is C4); only the two
// model resources the editor's lifecycle inserts survive (the egui viewport reads them in C4).
pub use canvas::{CanvasZoom, CurrentEditLevel, LevelStep};
pub use capture::EditorCapturePlugin;
// GTW-531: the up→down connector auto-pairing surface — exported so the prefab viewport commit
// (and the in-crate + integration round-trip tests) drive the real path.
pub use connector_pairing::{
    PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
};
pub use editor_map::EditorMap;
// GTW-464: the PREFAB size fields' view model — displayed spans derived FRESH from the session
// every egui pass (the session → fields reverse sync) + the kept clamp commit. Exported so the
// headless test asserts the exact model the panel renders from (the GTW-512 pattern).
pub use egui_shell::prefab::size_fields::SizeFieldSpans;
pub use hovered_cell::HoveredCell;
// GTW-512: only the `EditorMode` enum survives the egui swap (the `bevy_ui` tab / content markers
// are gone — the egui shell draws the tabs + branches the right panel in-UI).
pub use mode::EditorMode;
pub use placement::{
    EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement, classify,
    evaluate_placement, names_a_ladder,
};
pub use plugin::MapEditorPlugin;
// GTW-515: the prefab preview render-target resource + the owned pan-offset target (the zoom target
// is the kept `CanvasZoom`). Exported so the headless test asserts the state-scoped lifecycle.
pub use preview::{target::PreviewTarget, view::PreviewPan};
pub use right_panel::{GridSpanInput, SizeFieldAxis, ThemeDropdown};
// GTW-512: the save PROJECTION surface (debug-only, the v2 save path) — kept for the C4 save-control
// re-point + the in-crate save tests. The `bevy_ui` save controls themselves are the C4 child.
#[cfg(debug_assertions)]
pub use save::{
    SavePrefabError, editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab,
    write_prefab,
};
pub use session::MapEditorSession;
pub use state::EditorState;
pub use terrain_form::{
    ArmorInput, FootfallChoice, HpInput, SaveTerrainError, TerrainDraft, TerrainGraphicChoice,
    TerrainKindChoice, draft_to_terrain_def, serialize_terrain_def,
};
// The debug-only TERRAIN / THEME fs-write surface — kept for the C2 / C3 egui save-press re-point.
// `write_terrain_in` is the root-parameterized core: tests call it with a `tempfile::TempDir` root
// so they never write into the version-controlled `assets/` tree. `write_terrain` is the production
// thin wrapper (WORKSPACE_ASSETS_ROOT). Both are `cfg(debug_assertions)`-only.
#[cfg(debug_assertions)]
pub use terrain_form::{write_terrain, write_terrain_in};
#[cfg(debug_assertions)]
pub use theme_form::write_theme;
pub use theme_form::{
    SaveThemeError, ThemeDraft, draft_to_theme_def, floor_candidates, resolved_stats,
    serialize_theme_def, slab_floor_candidates, validate_for_save,
};
