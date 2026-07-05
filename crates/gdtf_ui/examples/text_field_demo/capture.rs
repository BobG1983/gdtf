//! Render-evidence capture: the self-screenshot request at [`SHOT_FRAME`] and the PNG
//! readback poll that gates the commit drive.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use crate::FrameCounter;

/// Resource holding the optional self-screenshot output path (from `GDTF_TEXTFIELD_SHOT`).
#[derive(Resource)]
pub(crate) struct ShotPath(Option<PathBuf>);

impl ShotPath {
    /// Wraps the optional output path parsed from the `GDTF_TEXTFIELD_SHOT` env var.
    pub(crate) const fn new(path: Option<PathBuf>) -> Self {
        Self(path)
    }
}

/// Tracks whether the screenshot was already requested this run.
#[derive(Resource, Default)]
pub(crate) struct ShotRequested(bool);

/// Tracks whether the PNG has landed on disk (the screenshot's async GPU readback completed).
#[derive(Resource, Default)]
pub(crate) struct ShotWritten(bool);

impl ShotWritten {
    /// Whether the PNG has been confirmed on disk (or the live run needs no PNG).
    pub(crate) const fn is_written(&self) -> bool {
        self.0
    }
}

/// The frame the demo requests the screenshot (after a layout settle; the GPU readback is
/// ASYNC, so the commit drive + exit happen only after the file lands).
const SHOT_FRAME: u32 = 16;

/// At [`SHOT_FRAME`] (if `GDTF_TEXTFIELD_SHOT` was set), requests a screenshot via the observer
/// API; the demo does NOT exit here — [`poll_for_png`] polls until the file appears, and only
/// THEN does [`drive_commits`](crate::commit_drive::drive_commits) kick the commits.
pub(crate) fn maybe_capture(
    frames: Res<FrameCounter>,
    shot: Res<ShotPath>,
    mut requested: ResMut<ShotRequested>,
    mut commands: Commands,
) {
    if requested.0 {
        return;
    }
    let Some(path) = shot.0.clone() else {
        return;
    };
    if frames.count() != SHOT_FRAME {
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    requested.0 = true;
}

/// Polls until the PNG appears (written by the `save_to_disk` observer after GPU readback) and
/// sets [`ShotWritten`]. With no shot path set, marks it written immediately so the live /
/// no-screenshot run still proceeds to the commit drive.
pub(crate) fn poll_for_png(
    shot: Res<ShotPath>,
    requested: Res<ShotRequested>,
    mut written: ResMut<ShotWritten>,
) {
    if written.0 {
        return;
    }
    let Some(path) = shot.0.as_ref() else {
        // No screenshot requested (live run): treat the render evidence as "done" so the
        // commit drive still runs.
        written.0 = true;
        return;
    };
    if !requested.0 {
        return;
    }
    if std::path::Path::new(path).exists() {
        info!(
            "text_field_demo: PNG written to {}, driving commits.",
            path.display()
        );
        written.0 = true;
    }
}
