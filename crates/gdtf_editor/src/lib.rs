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

mod app;
mod capture;
mod load;
mod palette;
mod plugin;
mod regions;
mod right_panel;
mod session;
mod state;
mod tile_atlas;

pub use app::MapEditorApp;
pub use capture::EditorCapturePlugin;
pub use palette::{PaletteRow, StatText};
pub use plugin::MapEditorPlugin;
pub use regions::{CanvasRegion, EditorShellRoot, LeftPaletteRegion, RightPanelRegion, StatRegion};
pub use right_panel::{GridSpanInput, SizeFieldAxis, ThemeDropdown};
pub use session::MapEditorSession;
pub use state::EditorState;
