//! The capture drive's per-frame SYSTEMS — the env-forced overrides
//! (`force_capture_*`) and the model drive (`drive_capture_*`) that stage a legible,
//! deterministic scene before the delegated `gdtf_screenshot` settle + shot.

use bevy::prelude::*;
use gdtf_battle_presenter::{TileRoles, ViewMode};
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, UuidThemeRegistry},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
    weapon::{WeaponName, WeaponRegistry},
};

use super::forced::{ForcedMode, ForcedTerrainKind, ForcedView, ForcedZoom};
use crate::{
    EditorMap, EditorMode, canvas::CanvasZoom, hovered_cell::HoveredCell,
    session::MapEditorSession, terrain_form::TerrainDraft, terrain_graphics::terrain_atlas_index,
};

/// The modest, legible grid edge the capture shrinks the canvas to before the shot — a `16 × 16`
/// (× 1 level) drawable area, so the shell + viewport read CLEARLY in the screenshot (the full
/// `60 × 60` grid is too dense). A framework layout const for the capture drive (clause-4 plumbing
/// carve-out).
pub(super) const SHOT_GRID_EDGE: u8 = 16;

/// The painted-block edge for the capture — a `BLOCK × BLOCK` square of painted cells near the
/// canvas top-left, large enough to read clearly in the shot. A framework plumbing const.
const BLOCK: i32 = 3;

/// The storey count the FULL-VIEW capture variant (GTW-532) uses — a `SHOT_GRID_EDGE² × 2` volume
/// so a distinct block on the UPPER storey (storey 1) reads in the `FullView` capture and is culled
/// in a ground-storey `DownToActive` capture. A framework plumbing const.
const SHOT_STOREYS_FULL: u8 = 2;

/// The upper storey the FULL-VIEW capture paints its distinct block on (storey 1) — visible only in
/// [`ViewMode::FullView`] (culled at the ground storey in the default view). A framework plumbing
/// const.
const UPPER_STOREY: u8 = 1;

/// `Update` (in `Editing`, capture-only): FORCE the [`EditorMode`] to the C2.4 [`ForcedMode`]
/// before the settle / screenshot — so the Screenshot-QA can capture a specific Workbench mode
/// (`GDTF_EDITOR_MODE=terrain|theme|prefab`). No-ops when no mode was forced (the resource is
/// absent) or the editor's mode already matches. Both borrows are `Option` (state-scoped —
/// bevy-traps #1); [`set_if_neq`](DetectChangesMut::set_if_neq) keeps an already-matching mode a
/// no-op (idempotent under the egui multipass re-run).
pub(super) fn force_capture_mode(
    forced: Option<Res<ForcedMode>>,
    mode: Option<ResMut<EditorMode>>,
) {
    let (Some(forced), Some(mut mode)) = (forced, mode) else {
        return;
    };
    mode.set_if_neq(**forced);
}

/// `Update` (in `Editing`, capture-only): pre-select the TERRAIN form's kind segment to the
/// GTW-574 [`ForcedTerrainKind`] before the settle / screenshot — driving the live
/// [`TerrainDraft`] through the SAME [`set_kind`](TerrainDraft::set_kind) setter the egui
/// segmented row commits. For the Emplacement kind it ALSO pre-selects the FIRST (sorted)
/// [`WeaponRegistry`] key when no mounted weapon is selected yet, so the registry-backed
/// dropdown captures POPULATED with a real weapon name (the AC-7 positive content — egui
/// closures never run headless, so the screenshot is the in-engine evidence).
///
/// No-ops when no kind was forced / the draft is absent (state-scoped — bevy-traps #1);
/// idempotent — the kind write is skipped once matching, and the weapon pre-select only fills
/// an EMPTY selection (a real selection is never overwritten).
pub(super) fn force_capture_terrain_kind(
    forced: Option<Res<ForcedTerrainKind>>,
    draft: Option<ResMut<TerrainDraft>>,
    weapons: Option<Res<WeaponRegistry>>,
) {
    let (Some(forced), Some(mut draft)) = (forced, draft) else {
        return;
    };
    if draft.kind() != **forced {
        draft.set_kind(**forced);
    }
    // Emplacement: fill an empty mounted-weapon selection from the live registry (sorted first
    // key — the same stable order the dropdown offers), so the combo shows a real weapon.
    if draft.kind() == crate::terrain_form::TerrainKindChoice::Emplacement
        && draft.mounted_weapon().is_none()
        && let Some(registry) = weapons
    {
        let mut names: Vec<&WeaponName> = registry.keys().collect();
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        if let Some(first) = names.first() {
            draft.set_mounted_weapon(Some((*first).clone()));
        }
    }
}

/// `Update` (in `Editing`, capture-only): FORCE the preview [`CanvasZoom`] to the C4.11
/// [`ForcedZoom`] before the settle / screenshot — so a SECOND capture proves the projection-scale
/// path renders at a non-identity zoom (catches an empty/black viewport at a scaled projection).
/// No-ops when no zoom was forced (the resource is absent) or the zoom already matches. Both
/// borrows are `Option` (state-scoped — bevy-traps #1); [`set_if_neq`](DetectChangesMut::set_if_neq)
/// keeps an already-matching zoom a no-op (idempotent — set-to-target). The
/// `apply_preview_view` system then drives the camera's `OrthographicProjection::scale` from it.
pub(super) fn force_capture_zoom(
    forced: Option<Res<ForcedZoom>>,
    zoom: Option<ResMut<CanvasZoom>>,
) {
    let (Some(forced), Some(mut zoom)) = (forced, zoom) else {
        return;
    };
    zoom.set_if_neq(**forced);
}

/// `Update` (in `Editing`, capture-only): FORCE the prefab [`ViewMode`] to the GTW-532
/// [`ForcedView`] before the settle / screenshot — so a SECOND capture proves the full-view toggle
/// changes the drawn storey stack. No-ops when no view was forced (the resource is absent) or the
/// view already matches. Both borrows are `Option` (state-scoped — bevy-traps #1);
/// [`set_if_neq`](DetectChangesMut::set_if_neq) keeps an already-matching view a no-op (idempotent
/// under the egui multipass re-run). The `redraw_preview_tiles` system then redraws the storey
/// stack from it (its `ViewMode::is_changed` trigger).
pub(super) fn force_capture_view(forced: Option<Res<ForcedView>>, view: Option<ResMut<ViewMode>>) {
    let (Some(forced), Some(mut view)) = (forced, view) else {
        return;
    };
    view.set_if_neq(**forced);
}

/// `Update` (in `Editing`, capture-only): shrink the [`MapEditorSession`] grid to a legible
/// [`SHOT_GRID_EDGE`]-square before the shot (the full `60 × 60` is too dense). Drives the session
/// directly (the SAME path the size selector commits through), so the shell / viewport re-extent.
/// Idempotent: once the grid matches the shot size it no-ops.
///
/// GTW-532: when the full-view variant is forced ([`ForcedView`] present) the grid gets
/// [`SHOT_STOREYS_FULL`] storeys instead of 1 so a distinct block painted on the UPPER storey is
/// visible in the `FullView` capture (and culled in a `DownToActive` capture at the ground storey).
pub(super) fn drive_capture_grid_size(
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
/// theme's terrain palette + the per-def atlas index (resolved THE WAY THE PRESENTER DOES, via
/// [`TileRoles`]), so it waits until those registries + the seeded theme resolve.
pub(super) fn drive_capture_selection(
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    roles: Option<Res<TileRoles>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(registry), Some(themes), Some(roles), Some(mut session)) =
        (registry, themes, roles, session)
    else {
        return;
    };
    if session.selected_tile().is_some() {
        return;
    }
    if let Some(paint_key) = distinct_paint_tile(&registry, &themes, &roles, &session) {
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
pub(super) fn drive_capture_paint_and_hover(
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

/// Resolve a terrain of the session's theme palette whose resolved atlas index DIFFERS from the
/// default-floor's, returning its [`TerrainUuid`] — so a painted cell reads visibly different in the
/// capture. The index resolution mirrors the presenter (via [`TileRoles`]).
fn distinct_paint_tile(
    registry: &TerrainDefRegistry,
    themes: &UuidThemeRegistry,
    roles: &TileRoles,
    session: &MapEditorSession,
) -> Option<TerrainUuid> {
    let theme = session.theme();
    let default_index = session
        .default_floor()
        .or_else(|| themes.default_floor(&theme))
        .and_then(|key| terrain_atlas_index(registry, roles, &key))
        .map(|index| *index)?;
    themes.terrain(&theme)?.iter().find_map(|key| {
        terrain_atlas_index(registry, roles, key)
            .map(|index| *index)
            .filter(|index| *index != default_index)
            .map(|_| *key)
    })
}
