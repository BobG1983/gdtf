//! QA / debug-only self-screenshot-then-exit affordance for the editor shell (GTW-417 AC4).
//!
//! OFF BY DEFAULT: with no `GDTF_EDITOR_SHOT` env var the editor launches normally —
//! interactive, no screenshot, no auto-exit. When `GDTF_EDITOR_SHOT=/abs/out.png` is set,
//! the editor — once it reaches [`Editing`](crate::EditorState::Editing) and the layout has
//! settled — captures the primary window to that PNG and exits cleanly. This lets QA
//! capture the empty shell unattended (AC4) while keeping the SHIPPED editor clean (the
//! reachable-overlay debug-gating precedent: an env-gated, inert-by-default affordance).
//!
//! Before the shot the capture drives the shell into a legible state: it shrinks the canvas to a
//! modest `16 × 16` grid (the full `60 × 60` is too dense to read — GTW-423 C5; `16` is the
//! smallest size that exposes the GTW-463 box-model wrap bug pre-fix), selects the first palette
//! tile (GTW-422 C5), PAINTS a small block of cells with a tile distinct from the default-floor,
//! and hovers a cell so the translucent ghost shows over it (GTW-426 C4) — so the captured frame
//! shows the populated palette, a highlighted row, the bottom-right stats, the dashed canvas
//! boundary + per-cell dashes + default-floor fill, AND a painted (non-default) block beside the
//! hover ghost.
//!
//! It mirrors the `gdtf_ui` `scroll_list_demo` capture mechanism: a
//! [`Screenshot::primary_window`] spawned with a [`save_to_disk`] observer, then a poll for
//! the PNG (the GPU readback is async) before writing [`AppExit`], with a frame cap so a
//! failed write never hangs.

use std::{env, path::PathBuf};

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    state::state::OnEnter,
    ui::widget::ImageNode,
};
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeCatalogRegistry, TileKey},
};
use gdtf_ui::ActiveButton;

use crate::{
    CanvasCell, EditorMap, EditorState, PaletteRow, canvas::follow_hover_ghost,
    session::MapEditorSession,
};

/// The modest, legible grid edge the capture shrinks the canvas to before the shot — a `16 × 16`
/// (× 1 level) drawable area, so the dashed boundary + per-cell dashes + the default-floor fill
/// read CLEARLY in the screenshot (the full `60 × 60` grid is too dense to make out — GTW-423 C5).
///
/// `16` is the smallest power-of-two where the old box-model bug manifests: the pre-fix formula
/// packed `floor(26 × 16 / 24) = 17` cells per row at width 16 instead of 16, producing right-
/// overflow and a ragged bottom-right. Post-fix (`BorderBox` footprint = 24 px), the formula packs
/// exactly 16 cells per row. The previous value of `8` was a "lucky fit" (`floor(26 × 8 / 24) = 8`
/// — the same result) and therefore masked the bug in prior QA captures.
///
/// A framework layout const for the capture drive (clause-4 plumbing carve-out).
const SHOT_GRID_EDGE: u8 = 16;

/// The env var that opts the capture affordance IN. Set it to an absolute PNG path; leave
/// it unset for a normal interactive launch.
const SHOT_ENV_VAR: &str = "GDTF_EDITOR_SHOT";

/// Frames to wait after entering [`Editing`](crate::EditorState) before requesting the
/// screenshot, so the four-region layout has laid out + drawn first.
const SETTLE_FRAMES: u32 = 12;

/// Frames to poll for the PNG before giving up (a safety cap so a failed write never hangs
/// the editor). At ~60 fps this is ~10 seconds.
const POLL_CAP: u32 = 600;

/// The resolved capture output path (from `GDTF_EDITOR_SHOT`). Present only when the
/// affordance is enabled; the plugin inserts it iff the env var is set.
#[derive(Resource, Deref)]
struct ShotPath(PathBuf);

impl ShotPath {
    /// Wrap the resolved capture path.
    const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// A count of frames elapsed since entering [`Editing`](crate::EditorState). Wrapping the
/// raw counter (no-bare-types rule 1) keeps the elapsed-frame value a named domain type
/// rather than a bare `u32`, so it can never be confused with the unrelated poll-frame count.
#[derive(Clone, Copy, Default, Deref)]
struct FrameCount(u32);

impl FrameCount {
    /// Advances the counter by one frame.
    const fn tick(&mut self) {
        self.0 += 1;
    }
}

/// Whether this run's screenshot has already been requested. A named flag type (no-bare-types
/// rule 1) so the "request fired" state reads as a domain value, not a bare `bool`.
#[derive(Clone, Copy, Default, Deref)]
struct ShotRequested(bool);

impl ShotRequested {
    /// Marks the screenshot as requested.
    const fn mark() -> Self {
        Self(true)
    }
}

/// Per-run capture progress: counts frames since entering `Editing`, and tracks whether the
/// screenshot has been requested. Both leaves are named newtypes ([`FrameCount`] /
/// [`ShotRequested`]) over their raw counters (no-bare-types).
#[derive(Resource, Default)]
struct CaptureProgress {
    /// Frames elapsed since entering `Editing` (reset on enter).
    frames:    FrameCount,
    /// Whether the screenshot has already been requested this run.
    requested: ShotRequested,
}

/// QA / debug-only screenshot-then-exit plugin for the editor shell.
///
/// Construct it via [`from_env`](EditorCapturePlugin::from_env): it reads
/// `GDTF_EDITOR_SHOT` ONCE and, when unset, [`build`](EditorCapturePlugin::build) registers
/// NOTHING — the affordance is indistinguishable from absent (inert by default). When set,
/// it wires the settle → capture → poll-then-exit systems and inserts the [`ShotPath`].
pub struct EditorCapturePlugin {
    /// The resolved capture path, or `None` when the env var was unset (plugin inert).
    path: Option<PathBuf>,
}

impl EditorCapturePlugin {
    /// Reads `GDTF_EDITOR_SHOT` once and builds the plugin. When the var is unset the
    /// plugin is inert (registers nothing); when set the value is the PNG output path.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            path: env::var(SHOT_ENV_VAR).ok().map(PathBuf::from),
        }
    }
}

impl Plugin for EditorCapturePlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self.path.clone() else {
            // Inert by default: no env var -> no systems, no resources, normal launch.
            return;
        };
        app.insert_resource(ShotPath::new(path))
            .init_resource::<CaptureProgress>()
            .add_systems(OnEnter(EditorState::Editing), reset_progress)
            .add_systems(
                Update,
                (
                    drive_capture_grid_size,
                    drive_capture_selection,
                    // Paint + write the hover BEFORE the live `follow_hover_ghost` reads it, so the
                    // manual `Interaction::Hovered` drives the real ghost-snap (the windowed
                    // `ui_focus_system` clears it each PreUpdate, so this re-asserts it every frame).
                    drive_capture_paint_and_ghost.before(follow_hover_ghost),
                    settle_then_capture,
                    poll_then_exit,
                )
                    .chain()
                    .run_if(in_state(EditorState::Editing)),
            );
    }
}

/// `OnEnter(Editing)`: reset the per-run capture counters so the settle window is measured
/// from the moment the shell scene comes up.
fn reset_progress(mut progress: ResMut<CaptureProgress>) {
    *progress = CaptureProgress::default();
}

/// `Update` (in `Editing`, capture-only): drive a tile SELECTION before the shot so the
/// captured frame shows the left palette POPULATED, a row HIGHLIGHTED, and the bottom-right
/// stats POPULATED (GTW-422 C5).
///
/// Runs every frame until a selection exists: once the palette rows are built (the `Update`
/// `sync_palette` has run), it picks the first [`PaletteRow`], writes its
/// [`TileKey`](gdtf_battle_sim::level::TileKey) into the session
/// ([`MapEditorSession::select_tile`]) and adds the [`ActiveButton`] highlight marker — exactly
/// what a real click does, but driven directly (the windowed `ui_focus_system` would clear a
/// synthesized `Interaction::Pressed` before the click handler sees it, so this writes the
/// selection state itself). The stat-region refresh then fires on the `is_changed` session.
/// Idempotent: once the session has a selected tile it no-ops.
fn drive_capture_selection(
    mut commands: Commands,
    rows: Query<(Entity, &PaletteRow)>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let Some(mut session) = session else {
        return;
    };
    if session.selected_tile().is_some() {
        return;
    }
    let Some((entity, row)) = rows.iter().next() else {
        return;
    };
    session.select_tile(row.tile().clone());
    commands.entity(entity).insert(ActiveButton);
}

/// `Update` (in `Editing`, capture-only): shrink the canvas to a legible [`SHOT_GRID_EDGE`]-square
/// grid before the shot so the dashed boundary + per-cell dashes + default-floor fill are clearly
/// visible (GTW-423 C5 — the full `60 × 60` grid is too dense to read). At `16 × 16` the old
/// box-model bug (GTW-463) was detectable (`floor(26 × 16 / 24) = 17 ≠ 16`); at the old `8 × 8`
/// it was not (`floor(26 × 8 / 24) = 8` — a lucky fit). The capture now validates at a
/// bug-exposing size.
///
/// Runs every frame until the grid is already at the shot size: rebuilds the session's
/// [`GridSize`] to `SHOT_GRID_EDGE × SHOT_GRID_EDGE × 1` via the validated [`GridSize::new`]
/// (fail-closed — a build error leaves the grid unchanged, never a panic). Drives the session
/// directly (the same path the GTW-421 size fields commit through), so [`sync_canvas`] re-extents
/// the canvas on the next frame. Idempotent: once the grid matches the shot size it no-ops.
fn drive_capture_grid_size(session: Option<ResMut<MapEditorSession>>) {
    let Some(mut session) = session else {
        return;
    };
    let current = session.grid_size();
    if *current.width() == SHOT_GRID_EDGE && *current.height() == SHOT_GRID_EDGE {
        return;
    }
    if let Ok(size) = GridSize::new(
        GridWidth::new(SHOT_GRID_EDGE),
        GridHeight::new(SHOT_GRID_EDGE),
        GridLevels::new(1),
    ) {
        session.set_grid_size(size);
    }
}

/// `Update` (in `Editing`, capture-only): PAINT a few cells + hover a cell so the captured frame
/// demonstrates the GTW-426 interactivity — at least one painted (non-default) cell AND the
/// translucent ghost over a hovered cell (GTW-426 C4).
///
/// Two halves:
///
/// 1. PAINT (once): resolves a paint tile DISTINCT from the theme default-floor (so the painted
///    cells read clearly), records a small block of cells in the [`EditorMap`] (clamped — C3), and
///    rewrites those cells' [`ImageNode`] atlas indices to the painted tile — the same effect the
///    live [`paint_cell`](crate::canvas::paint_cell) produces, driven directly (the windowed
///    `ui_focus_system` would clear a synthesized `Interaction::Pressed`, the
///    `drive_capture_selection` precedent). Idempotent (no-ops once the model holds paints).
/// 2. HOVER (every frame): writes [`Interaction::Hovered`] on a cell adjacent to the painted block
///    AND sets it as the session's selected tile, so the REAL
///    [`follow_hover_ghost`](crate::canvas::follow_hover_ghost) — ordered AFTER this — shows + snaps
///    the ghost. This drives the live ghost path rather than poking the ghost directly: it is
///    ordered before `follow_hover_ghost` (which runs in PreUpdate-fed `Update`), so the manual
///    `Hovered` survives into the follow read. Running it every frame re-asserts the hover after
///    the windowed `ui_focus_system` clears it each `PreUpdate`.
fn drive_capture_paint_and_ghost(
    registry: Option<Res<ThemeCatalogRegistry>>,
    mut session: Option<ResMut<MapEditorSession>>,
    mut map: Option<ResMut<EditorMap>>,
    mut cells: Query<(&CanvasCell, &mut ImageNode, &mut Interaction)>,
) {
    let (Some(registry), Some(session), Some(map)) = (registry, session.as_mut(), map.as_mut())
    else {
        return;
    };
    // A tile distinct from the theme default-floor, so the painted cells read as different.
    let Some((paint_key, paint_index)) = distinct_paint_tile(&registry, session) else {
        return;
    };
    // Keep that tile selected so the ghost previews it (and the palette stat region matches).
    session.select_tile(paint_key.clone());
    let size = session.grid_size();

    // PAINT a small block once (idempotent — skip if the model already holds paints).
    if map.painted_count() == 0 {
        paint_block(map, &mut cells, &paint_key, paint_index, size);
    }

    // HOVER a cell adjacent to the painted block so the live follow shows the ghost there.
    let hover = Cell::new(BLOCK, 1);
    for (canvas_cell, _, mut interaction) in &mut cells {
        if canvas_cell.cell() == hover {
            *interaction = Interaction::Hovered;
        }
    }
}

/// The painted block edge for the capture — a `BLOCK × BLOCK` square of painted cells near the
/// canvas top-left, large enough to read clearly in the shot. A framework plumbing const.
const BLOCK: i32 = 3;

/// Resolve a catalog tile of the session's theme whose atlas index DIFFERS from the default-floor,
/// returning its key + index — so a painted cell reads visibly different in the capture.
fn distinct_paint_tile(
    registry: &ThemeCatalogRegistry,
    session: &MapEditorSession,
) -> Option<(TileKey, usize)> {
    let catalog = registry.catalog(session.theme())?;
    let default_index = catalog.default_floor().map(|tile| *tile.atlas_index)?;
    catalog.tiles().find_map(|(key, tile)| {
        (*tile.atlas_index != default_index).then(|| (key.clone(), *tile.atlas_index))
    })
}

/// Paint a `BLOCK × BLOCK` block of cells into the model + redraw their sprites. Mirrors the live
/// `paint_cell` effect (model write + in-place sprite redraw) for each cell in the block, driven
/// directly (no `Interaction`).
fn paint_block(
    map: &mut EditorMap,
    cells: &mut Query<(&CanvasCell, &mut ImageNode, &mut Interaction)>,
    paint_key: &TileKey,
    paint_index: usize,
    size: GridSize,
) {
    let block: Vec<Cell> = (0..BLOCK)
        .flat_map(|x| (0..BLOCK).map(move |y| Cell::new(x, y)))
        .collect();
    for cell in block {
        if !map.paint(cell, paint_key.clone(), size) {
            continue;
        }
        for (canvas_cell, mut node, _) in cells.iter_mut() {
            if canvas_cell.cell() == cell
                && let Some(atlas) = node.texture_atlas.as_mut()
            {
                atlas.index = paint_index;
            }
        }
    }
}

/// `Update` (in `Editing`): after [`SETTLE_FRAMES`], requests one primary-window screenshot
/// with a [`save_to_disk`] observer (once). The readback is async, so this does NOT exit
/// here — [`poll_then_exit`] waits for the file.
fn settle_then_capture(
    shot: Res<ShotPath>,
    mut progress: ResMut<CaptureProgress>,
    mut commands: Commands,
) {
    if *progress.requested {
        return;
    }
    progress.frames.tick();
    if *progress.frames < SETTLE_FRAMES {
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk((**shot).clone()));
    progress.requested = ShotRequested::mark();
}

/// `Update` (in `Editing`): once the screenshot has been requested, polls each frame until
/// the PNG appears on disk, then writes [`AppExit::Success`]. A [`POLL_CAP`] safety net
/// prevents a failed write from hanging the editor.
fn poll_then_exit(
    shot: Res<ShotPath>,
    progress: Res<CaptureProgress>,
    mut poll_frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    if !*progress.requested {
        return;
    }
    *poll_frames += 1;
    if std::path::Path::new(&**shot).exists() {
        info!("map_editor: shell screenshot written to {}", shot.display());
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= POLL_CAP {
        warn!(
            "map_editor: screenshot PNG not found after {POLL_CAP} poll frames; giving up. Was \
             GDTF_EDITOR_SHOT set to a writable path?",
        );
        exit.write(AppExit::Success);
    }
}
