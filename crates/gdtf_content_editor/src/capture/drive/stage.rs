//! The scene STAGING drives (`drive_capture_*`) — shrink the grid, select a distinct
//! palette tile, and paint the legible block(s) the capture reads, all on the MODEL (the
//! same paths the live controls commit through).

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, UuidThemeRegistry},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::super::forced::ForcedView;
use crate::{
    EditorMap, hovered_cell::HoveredCell, session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

/// The modest, legible grid edge the capture shrinks the canvas to before the shot — a `16 × 16`
/// (× 1 level) drawable area, so the shell + viewport read CLEARLY in the screenshot (the full
/// `60 × 60` grid is too dense). A framework layout const for the capture drive (clause-4 plumbing
/// carve-out).
pub(in crate::capture) const SHOT_GRID_EDGE: u8 = 16;
/// The painted-block edge for the capture — a `BLOCK × BLOCK` square of painted cells near the
/// canvas top-left, large enough to read clearly in the shot. A framework plumbing const.
const BLOCK: i32 = 3;

/// The storey count a FORCED-VIEW capture variant (GTW-532 `full` / GTW-594 `isolate`) uses — a
/// `SHOT_GRID_EDGE² × 2` volume so a distinct block on the UPPER storey (storey 1) reads in the
/// staged capture and is culled in a plain ground-storey capture. A framework plumbing const.
const SHOT_STOREYS_FULL: u8 = 2;

/// The upper storey a forced-view capture paints its distinct block on (storey 1) — re-shown by
/// [`ViewMode::FullView`] in the `full` variant, and the ACTIVE (edit) storey of the GTW-594
/// `isolate` variant. A framework plumbing const.
const UPPER_STOREY: u8 = 1;
/// `Update` (in `Editing`, capture-only): shrink the [`MapEditorSession`] grid to a legible
/// [`SHOT_GRID_EDGE`]-square before the shot (the full `60 × 60` is too dense). Drives the session
/// directly (the SAME path the size selector commits through), so the shell / viewport re-extent.
/// Idempotent: once the grid matches the shot size it no-ops.
///
/// GTW-532: when the full-view variant is forced ([`ForcedView`] present) the grid gets
/// [`SHOT_STOREYS_FULL`] storeys instead of 1 so a distinct block painted on the UPPER storey is
/// visible in the `FullView` capture (and culled in a `DownToActive` capture at the ground storey).
pub(in crate::capture) fn drive_capture_grid_size(
    forced_view: Option<Res<ForcedView>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let Some(mut session) = session else {
        return;
    };
    let levels = if forced_view.is_some() {
        SHOT_STOREYS_FULL
    } else {
        1
    };
    let current = session.grid_size();
    if *current.width() == SHOT_GRID_EDGE
        && *current.height() == SHOT_GRID_EDGE
        && *current.levels() == levels
    {
        return;
    }
    if let Ok(size) = GridSize::new(
        GridWidth::new(SHOT_GRID_EDGE),
        GridHeight::new(SHOT_GRID_EDGE),
        GridLevels::new(levels),
    ) {
        session.set_grid_size(size);
    }
}

/// `Update` (in `Editing`, capture-only): SELECT a distinct (non-default-floor) palette tile in the
/// [`MapEditorSession`] before the shot — exactly what a real palette click does, driven directly.
/// Idempotent: once the session has a selected tile it no-ops. Resolving a distinct tile needs the
/// theme's terrain palette + the per-def sprite resolution (resolved THE WAY THE PRESENTER DOES,
/// via [`terrain_sprite_def`] over the GTW-663 [`SpriteDefRegistry`] — GTW-665), so it waits until
/// those registries + the seeded theme resolve.
pub(in crate::capture) fn drive_capture_selection(
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    sprites: Option<Res<SpriteDefRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(registry), Some(themes), Some(sprites), Some(mut session)) =
        (registry, themes, sprites, session)
    else {
        return;
    };
    if session.selected_tile().is_some() {
        return;
    }
    if let Some(paint_key) = distinct_paint_tile(&registry, &themes, &sprites, &session) {
        session.select_tile(paint_key);
    }
}

/// `Update` (in `Editing`, capture-only): PAINT a small block of cells into the [`EditorMap`] and
/// write the [`HoveredCell`] model so the captured frame shows a painted (non-default) block beside
/// the preview ghost (C1.5).
///
/// Two halves, both on the MODEL (no `bevy_ui`):
///
/// 1. PAINT (once): records a `BLOCK × BLOCK` block in the [`EditorMap`] with the session's selected
///    tile, clamped to the current grid ([`EditorMap::paint`]). Idempotent — skip if anything is
///    already painted.
/// 2. HOVER (every frame): writes [`HoveredCell`] to a cell beside the block (on the ground storey),
///    so the egui preview ghost (C4) renders over it. Written every frame for parity with the live
///    hover (a real cursor would re-assert it continuously).
pub(in crate::capture) fn drive_capture_paint_and_hover(
    forced_view: Option<Res<ForcedView>>,
    session: Option<Res<MapEditorSession>>,
    mut map: Option<ResMut<EditorMap>>,
    mut hovered: Option<ResMut<HoveredCell>>,
) {
    let (Some(session), Some(map), Some(hovered)) = (session, map.as_mut(), hovered.as_mut())
    else {
        return;
    };
    let Some(paint_key) = session.selected_tile() else {
        return;
    };
    let size = session.grid_size();

    // PAINT a small block once (idempotent — skip if the model already holds paints). In the
    // GTW-532 FullView variant, ALSO paint a distinct block on the UPPER storey (storey 1) — visible
    // only in FullView (culled at the ground storey in the default down-to-active view), so the two
    // captures visibly differ.
    if map.painted_count() == 0 {
        for cell in block_cells() {
            map.paint(cell, paint_key, size);
        }
        if forced_view.is_some() {
            let upper = Level::new(UPPER_STOREY);
            for cell in upper_block_cells() {
                map.paint_at(CellLevel::new(cell, upper), paint_key, size);
            }
        }
    }

    // Drive the preview hover on a cell beside the block (on the ground storey).
    hovered.set(Cell::new(BLOCK, 1), Level::new(0));
}

/// The `BLOCK × BLOCK` block of ground-plane cells the capture paints — near the canvas top-left.
fn block_cells() -> Vec<Cell> {
    (0..BLOCK)
        .flat_map(|x| (0..BLOCK).map(move |y| Cell::new(x, y)))
        .collect()
}

/// The `BLOCK × BLOCK` block the FULL-VIEW capture paints on the UPPER storey (GTW-532) — OFFSET
/// from the ground block so both read in the `FullView` capture (the upper block draws in front by
/// the per-storey z), and the upper block is culled in a ground-storey `DownToActive` capture.
fn upper_block_cells() -> Vec<Cell> {
    (0..BLOCK)
        .flat_map(|x| (0..BLOCK).map(move |y| Cell::new(x + BLOCK + 1, y)))
        .collect()
}

/// Resolve a terrain of the session's theme palette whose resolved SPRITE DEF differs from the
/// default-floor's, returning its [`TerrainUuid`] — so a painted cell reads visibly different in the
/// capture. The resolution mirrors the presenter (via [`terrain_sprite_def`] — GTW-665; a different
/// def means different authored pixels, the successor of the retired atlas-index-differs check).
fn distinct_paint_tile(
    registry: &TerrainDefRegistry,
    themes: &UuidThemeRegistry,
    sprites: &SpriteDefRegistry,
    session: &MapEditorSession,
) -> Option<TerrainUuid> {
    let theme = session.theme();
    let default_def = session
        .default_floor()
        .or_else(|| themes.default_floor(&theme))
        .and_then(|key| terrain_sprite_def(registry, sprites, &key))?;
    themes.terrain(&theme)?.iter().find_map(|key| {
        terrain_sprite_def(registry, sprites, key)
            .filter(|def| *def != default_def)
            .map(|_| *key)
    })
}
