//! The map-editor **central canvas** — the drawable cell grid that fills the GTW-417
//! [`CanvasRegion`](crate::CanvasRegion) (GTW-423).
//!
//! Once a size is set (the GTW-421 [`MapEditorSession::grid_size`]), the canvas draws the
//! drawable area as a `width × height` grid of cells, each PRE-FILLED with the current
//! theme's default-floor tile SPRITE (C3), bounded by a DASHED boundary (C1) and ruled with
//! PER-CELL dimmed dashes (C2). It LIVE-updates on theme / size change (C4): a theme switch
//! repaints every cell with the new default floor, a size change re-extents the grid.
//!
//! ## Scope: one x/y storey at a time, with a level selector (GTW-500)
//!
//! The canvas draws ONE x/y storey slice at a time — the grid's
//! [`width`](gdtf_battle_sim::level::GridSize::width) ×
//! [`height`](gdtf_battle_sim::level::GridSize::height) at the
//! [`CurrentEditLevel`](level_nav::CurrentEditLevel) storey. GTW-500 added the storey SELECTOR the
//! GTW-423 canvas lacked: the [`level_nav`] submodule steps [`CurrentEditLevel`] up/down via the
//! keyboard and chrome buttons, clamped to the prefab's `[0, levels-1]` range, and the render /
//! paint / ghost all read it, so stepping the level changes the drawn slice live.
//!
//! ## Chosen scale model (the GTW-423 open design decision)
//!
//! Each cell is rendered as a UI [`ImageNode`](bevy::ui::widget::ImageNode) of fixed edge
//! [`CANVAS_CELL_PX`](types::CANVAS_CELL_PX) inside the [`CanvasRegion`](crate::CanvasRegion).
//! Because the full `60 × 60` grid (1440 px at that constant) far exceeds the centre column,
//! the canvas grid is WRAPPED in a `gdtf_ui`
//! [`spawn_scroll_list`](gdtf_ui::spawn_scroll_list) — so it SCROLLS rather than squeezing.
//! The grid hangs in the returned [`ScrollListArea`](gdtf_ui::ScrollListArea) (the clipping,
//! scrolling viewport — NOT the scroll-list grid root frame the marker rides — the GTW-421
//! parenting rule).
//!
//! ## Lightweight dash mechanism (the C1/C2 perf clause)
//!
//! No extra nodes per cell: the per-cell dimmed dashes (C2) are each cell's own thin
//! [`Node::border`](bevy::ui::Node::border) painted a dimmed [`BorderColor`](bevy::ui::BorderColor);
//! the dashed boundary (C1) is a SINGLE thicker border on the grid container node. So a `w × h`
//! grid is exactly `w × h` cell [`ImageNode`](bevy::ui::widget::ImageNode)s plus one container —
//! never 2+ extra nodes per cell.
//!
//! ## Rebuild model (the ui-mutate-not-respawn carve-out)
//!
//! A theme/size change DESPAWNS the [`CanvasRoot`] subtree and rebuilds it. A full rebuild
//! (rather than a per-cell mutate) is the logged choice for the canvas: a size change alters
//! the cell COUNT (entities must be added/removed), and a full teardown keeps the boundary +
//! per-cell-dash + fill invariants in one builder. The rebuild is gated on a [`Local`](bevy::prelude::Local)
//! tracker of the `(grid_size, theme)` it was last built for, so an unrelated session mutation
//! (a palette selection) never triggers a needless rebuild.
//!
//! ## Module layout
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`types`] | Component markers, value types, layout consts, node builders, cell spawner, shared tile-index resolver |
//! | [`sync`]  | `sync_canvas` — initial build + theme/size-change rebuild |
//! | [`scroll`]| `spawn_canvas_scroll` — wraps the [`CanvasRegion`](crate::CanvasRegion) in a scroll list |
//! | [`paint`] | `paint_cell` — click-to-paint, writes [`EditorMap`](crate::EditorMap) + redraws sprite |
//! | [`ghost`] | `spawn_hover_ghost` / `follow_hover_ghost` — translucent preview ghost |
//! | [`level_nav`] | `CurrentEditLevel` + level up/down nav (keys + chrome buttons + readout) — GTW-500 C1 |
//! | [`center`] | `center_canvas` — centres the grid in the viewport when it fits — GTW-500 C2 |
//! | [`zoom`]  | `CanvasZoom` + mouse-wheel zoom (cell-size re-layout, cursor-anchored) — GTW-500 C3 |
//! | [`zoom_chrome`] | The clickable `Zoom n%` readout + its refresh + the zoom reset — GTW-500 C3 |
//! | [`tests`] | In-crate layout-guard unit tests |

mod center;
mod ghost;
mod level_nav;
mod paint;
mod scroll;
mod sync;
mod types;
mod zoom;
mod zoom_chrome;

#[cfg(test)]
mod tests;

// Public re-exports consumed by lib.rs (the external surface is unchanged).
// Crate-internal re-exports consumed by plugin.rs (system references).
pub(crate) use center::center_canvas;
pub(crate) use ghost::{follow_hover_ghost, spawn_hover_ghost};
pub use level_nav::{CurrentEditLevel, LevelNavButton, LevelReadout};
pub(crate) use level_nav::{
    clamp_level_to_grid, level_nav_buttons, level_nav_hotkeys, refresh_level_readout,
    spawn_level_nav,
};
pub(crate) use paint::paint_cell;
pub(crate) use scroll::spawn_canvas_scroll;
pub(crate) use sync::sync_canvas;
pub use types::{CanvasCell, CanvasExtent, CanvasGhost, CanvasRoot, CanvasScroll};
pub use zoom::CanvasZoom;
pub(crate) use zoom::{apply_canvas_zoom, read_zoom_wheel};
pub use zoom_chrome::ZoomReadout;
pub(crate) use zoom_chrome::{refresh_zoom_readout, reset_zoom_button, spawn_zoom_chrome};
