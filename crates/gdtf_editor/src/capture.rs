//! QA / debug-only self-screenshot-then-exit affordance for the editor shell (GTW-417 AC4).
//!
//! OFF BY DEFAULT: with no `GDTF_EDITOR_SHOT` env var the editor launches normally —
//! interactive, no screenshot, no auto-exit. When `GDTF_EDITOR_SHOT=/abs/out.png` is set,
//! the editor — once it reaches [`Editing`](crate::EditorState::Editing) and the layout has
//! settled — captures the primary window to that PNG and exits cleanly. This lets QA
//! capture the empty shell unattended (AC4) while keeping the SHIPPED editor clean (the
//! reachable-overlay debug-gating precedent: an env-gated, inert-by-default affordance).
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

use crate::EditorState;

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
                (settle_then_capture, poll_then_exit)
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
