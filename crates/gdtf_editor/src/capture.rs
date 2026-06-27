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
//! modest `8 × 8` grid (the full `60 × 60` is too dense to read — GTW-423 C5) and selects the first
//! palette tile (GTW-422 C5), so the captured frame shows the populated palette, a highlighted row,
//! the bottom-right stats, and the dashed canvas boundary + per-cell dashes + default-floor fill.
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
};
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_ui::ActiveButton;

use crate::{EditorState, PaletteRow, session::MapEditorSession};

/// The modest, legible grid edge the capture shrinks the canvas to before the shot — an `8 × 8`
/// (× 1 level) drawable area, so the dashed boundary + per-cell dashes + the default-floor fill
/// read CLEARLY in the screenshot (the full `60 × 60` grid is too dense to make out — GTW-423 C5).
/// A framework layout const for the capture drive (clause-4 plumbing carve-out).
const SHOT_GRID_EDGE: u8 = 8;

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
/// visible (GTW-423 C5 — the full `60 × 60` grid is too dense to read).
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
