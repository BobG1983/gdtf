//! QA / debug-only self-screenshot-then-exit affordance for the editor (GTW-417 AC4; re-pointed
//! onto the MODEL in the GTW-512 egui swap).
//!
//! OFF BY DEFAULT: with no `GDTF_EDITOR_SHOT` env var the editor launches normally — interactive, no
//! screenshot, no auto-exit. When `GDTF_EDITOR_SHOT=/abs/out.png` is set, the editor — once it
//! reaches [`Editing`](crate::EditorState::Editing) and the egui layout has settled — captures the
//! primary window to that PNG and exits cleanly. This lets the gate's Screenshot-QA phase capture the
//! egui shell unattended while keeping the SHIPPED editor clean (the env-gated, inert-by-default
//! affordance precedent).
//!
//! ## GTW-513 C2.4: capture mode-force
//!
//! A second, optional env var — `GDTF_EDITOR_MODE` (`terrain` | `theme` | `prefab`,
//! case-insensitive) — FORCES the [`EditorMode`](crate::EditorMode) before the settle / screenshot,
//! so a QA run can capture a SPECIFIC Workbench mode (e.g. the TERRAIN form). It is honored ONLY
//! when the capture affordance itself is enabled (`GDTF_EDITOR_SHOT` set); unset or an unrecognized
//! value keeps the editor's default mode (the pre-C2.4 behavior). Example:
//! `GDTF_EDITOR_SHOT=/abs/terrain.png GDTF_EDITOR_MODE=terrain cargo run -p gdtf_content_editor_bin`.
//!
//! ## GTW-512 C1.5: re-pointed onto the model
//!
//! The capture CORE is Bevy-native + UI-agnostic — [`Screenshot::primary_window`] +
//! [`save_to_disk`] + the settle-frames + the poll-then-exit [`AppExit`] write — so it is KEPT
//! verbatim. Its DRIVE was re-pointed: the pre-egui drive read/wrote `bevy_ui`
//! `Interaction`/`ImageNode`/`ActiveButton`/`PaletteRow`/`CanvasCell` entities (which no longer exist
//! under egui) to populate the palette / paint cells / hover a cell. The egui shell has no such
//! entities, so the drive now mutates the MODEL resources directly — exactly the state a real click
//! produces — which the egui shell + the (C4) viewport then render:
//!
//! - the [`MapEditorSession`] grid size is shrunk to a legible [`SHOT_GRID_EDGE`]-square,
//! - a distinct (non-default-floor) palette tile is SELECTED ([`MapEditorSession::select_tile`]),
//! - a small block of cells is PAINTED into the [`EditorMap`] ([`EditorMap::paint`]),
//! - a cell beside the block is HOVERED via the new [`HoveredCell`] model (C1.5) — so the preview
//!   ghost is QA-able headlessly without any `bevy_ui` plumbing.

use std::{env, path::PathBuf};

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    state::state::OnEnter,
};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridLevels, GridSize, GridWidth, UuidThemeRegistry},
    metric::Level,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{
    EditorMap, EditorMode, EditorState, hovered_cell::HoveredCell, session::MapEditorSession,
    terrain_graphics::terrain_atlas_index,
};

/// The modest, legible grid edge the capture shrinks the canvas to before the shot — a `16 × 16`
/// (× 1 level) drawable area, so the shell + viewport read CLEARLY in the screenshot (the full
/// `60 × 60` grid is too dense). A framework layout const for the capture drive (clause-4 plumbing
/// carve-out).
const SHOT_GRID_EDGE: u8 = 16;

/// The env var that opts the capture affordance IN. Set it to an absolute PNG path; leave it unset
/// for a normal interactive launch.
const SHOT_ENV_VAR: &str = "GDTF_EDITOR_SHOT";

/// The env var that FORCES the [`EditorMode`] before the capture (C2.4) — so the Screenshot-QA can
/// capture a SPECIFIC Workbench mode (`terrain` | `theme` | `prefab`, case-insensitive). Unset (or
/// an unrecognized value) keeps the default mode the editor opened in. Honored only when the
/// capture affordance itself is enabled (`GDTF_EDITOR_SHOT` set).
const MODE_ENV_VAR: &str = "GDTF_EDITOR_MODE";

/// Frames to wait after entering [`Editing`](crate::EditorState) before requesting the screenshot,
/// so the egui shell has laid out + drawn first.
const SETTLE_FRAMES: u32 = 12;

/// Frames to poll for the PNG before giving up (a safety cap so a failed write never hangs the
/// editor). At ~60 fps this is ~10 seconds.
const POLL_CAP: u32 = 600;

/// The painted-block edge for the capture — a `BLOCK × BLOCK` square of painted cells near the
/// canvas top-left, large enough to read clearly in the shot. A framework plumbing const.
const BLOCK: i32 = 3;

/// The resolved capture output path (from `GDTF_EDITOR_SHOT`). Present only when the affordance is
/// enabled; the plugin inserts it iff the env var is set.
#[derive(Resource, Deref)]
struct ShotPath(PathBuf);

impl ShotPath {
    /// Wrap the resolved capture path.
    const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// The capture's FORCED [`EditorMode`] (C2.4 — from `GDTF_EDITOR_MODE`), inserted only when the
/// env var named a recognized mode. A named wrapper (no-bare-types) so the forced mode reads as a
/// domain value rather than a bare `EditorMode` resource colliding with the editor's own one.
#[derive(Resource, Clone, Copy, Deref)]
struct ForcedMode(EditorMode);

impl ForcedMode {
    /// Parse a `GDTF_EDITOR_MODE` value (case-insensitive `terrain` | `theme` | `prefab`) into a
    /// forced mode, or [`None`] for an unset / unrecognized value (the capture keeps the editor's
    /// default mode).
    fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "terrain" => Some(Self(EditorMode::Terrain)),
            "theme" => Some(Self(EditorMode::Theme)),
            "prefab" => Some(Self(EditorMode::Prefab)),
            _ => None,
        }
    }
}

/// A count of frames elapsed since entering [`Editing`](crate::EditorState). Wrapping the raw
/// counter (no-bare-types rule 1) keeps the elapsed-frame value a named domain type rather than a
/// bare `u32`.
#[derive(Clone, Copy, Default, Deref)]
struct FrameCount(u32);

impl FrameCount {
    /// Advances the counter by one frame.
    const fn tick(&mut self) {
        self.0 += 1;
    }
}

/// Whether this run's screenshot has already been requested. A named flag type (no-bare-types rule
/// 1) so the "request fired" state reads as a domain value, not a bare `bool`.
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

/// QA / debug-only screenshot-then-exit plugin for the editor.
///
/// Construct it via [`from_env`](EditorCapturePlugin::from_env): it reads `GDTF_EDITOR_SHOT` ONCE
/// and, when unset, [`build`](EditorCapturePlugin::build) registers NOTHING — the affordance is
/// indistinguishable from absent (inert by default). When set, it wires the model-drive → settle →
/// capture → poll-then-exit systems and inserts the [`ShotPath`].
pub struct EditorCapturePlugin {
    /// The resolved capture path, or `None` when the env var was unset (plugin inert).
    path:        Option<PathBuf>,
    /// The forced capture mode (C2.4 — from `GDTF_EDITOR_MODE`), or `None` to keep the editor's
    /// default mode. Read once at construction.
    forced_mode: Option<ForcedMode>,
}

impl EditorCapturePlugin {
    /// Reads `GDTF_EDITOR_SHOT` (+ the optional `GDTF_EDITOR_MODE`) once and builds the plugin. When
    /// `GDTF_EDITOR_SHOT` is unset the plugin is inert (registers nothing); when set the value is the
    /// PNG output path, and `GDTF_EDITOR_MODE` (if a recognized `terrain` | `theme` | `prefab`)
    /// forces the captured Workbench mode (C2.4).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            path:        env::var(SHOT_ENV_VAR).ok().map(PathBuf::from),
            forced_mode: env::var(MODE_ENV_VAR)
                .ok()
                .as_deref()
                .and_then(ForcedMode::from_env_value),
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
            .add_systems(OnEnter(EditorState::Editing), reset_progress);
        // C2.4: insert the forced-mode resource (so a QA run can capture a SPECIFIC mode) only when
        // GDTF_EDITOR_MODE named a recognized mode; unset keeps the editor's default mode.
        if let Some(forced) = self.forced_mode {
            app.insert_resource(forced);
        }
        app.add_systems(
            Update,
            (
                force_capture_mode,
                drive_capture_grid_size,
                drive_capture_selection,
                drive_capture_paint_and_hover,
                settle_then_capture,
                poll_then_exit,
            )
                .chain()
                .run_if(in_state(EditorState::Editing)),
        );
    }
}

/// `Update` (in `Editing`, capture-only): FORCE the [`EditorMode`] to the C2.4 [`ForcedMode`]
/// before the settle / screenshot — so the Screenshot-QA can capture a specific Workbench mode
/// (`GDTF_EDITOR_MODE=terrain|theme|prefab`). No-ops when no mode was forced (the resource is
/// absent) or the editor's mode already matches. Both borrows are `Option` (state-scoped —
/// bevy-traps #1); [`set_if_neq`](DetectChangesMut::set_if_neq) keeps an already-matching mode a
/// no-op (idempotent under the egui multipass re-run).
fn force_capture_mode(forced: Option<Res<ForcedMode>>, mode: Option<ResMut<EditorMode>>) {
    let (Some(forced), Some(mut mode)) = (forced, mode) else {
        return;
    };
    mode.set_if_neq(**forced);
}

/// `OnEnter(Editing)`: reset the per-run capture counters so the settle window is measured from the
/// moment the editing scene comes up.
fn reset_progress(mut progress: ResMut<CaptureProgress>) {
    *progress = CaptureProgress::default();
}

/// `Update` (in `Editing`, capture-only): shrink the [`MapEditorSession`] grid to a legible
/// [`SHOT_GRID_EDGE`]-square before the shot (the full `60 × 60` is too dense). Drives the session
/// directly (the SAME path the size selector commits through), so the shell / viewport re-extent.
/// Idempotent: once the grid matches the shot size it no-ops.
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

/// `Update` (in `Editing`, capture-only): SELECT a distinct (non-default-floor) palette tile in the
/// [`MapEditorSession`] before the shot — exactly what a real palette click does, driven directly.
/// Idempotent: once the session has a selected tile it no-ops. Resolving a distinct tile needs the
/// theme's terrain palette + the per-def atlas index (resolved THE WAY THE PRESENTER DOES, via
/// [`TileRoles`]), so it waits until those registries + the seeded theme resolve.
fn drive_capture_selection(
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
fn drive_capture_paint_and_hover(
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

    // PAINT a small block once (idempotent — skip if the model already holds paints).
    if map.painted_count() == 0 {
        for cell in block_cells() {
            map.paint(cell, paint_key, size);
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

/// `Update` (in `Editing`): after [`SETTLE_FRAMES`], requests one primary-window screenshot with a
/// [`save_to_disk`] observer (once). The readback is async, so this does NOT exit here —
/// [`poll_then_exit`] waits for the file.
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

/// `Update` (in `Editing`): once the screenshot has been requested, polls each frame until the PNG
/// appears on disk, then writes [`AppExit::Success`]. A [`POLL_CAP`] safety net prevents a failed
/// write from hanging the editor.
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
        info!(
            "gdtf_content_editor: editor screenshot written to {}",
            shot.display()
        );
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= POLL_CAP {
        warn!(
            "gdtf_content_editor: screenshot PNG not found after {POLL_CAP} poll frames; giving up. Was \
             GDTF_EDITOR_SHOT set to a writable path?",
        );
        exit.write(AppExit::Success);
    }
}
