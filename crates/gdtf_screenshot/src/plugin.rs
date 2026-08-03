//! Bevy plugin that enables env-driven or path-driven auto capture.

use std::env;

use bevy::prelude::*;

use crate::{
    path::{CapturePath, parse_shot_path},
    settle::{PollCap, SettleFrames},
    trigger::{CaptureProgress, poll_then_exit, settle_then_capture},
};

/// Captures a screenshot after settle frames, then exits when the file lands.
///
/// Inactive when no path is configured (`from_env` missing/blank).
pub struct ScreenshotCapturePlugin {
    path:   Option<CapturePath>,
    settle: SettleFrames,
    poll:   PollCap,
}

impl ScreenshotCapturePlugin {
    /// Read the output path from an environment variable.
    #[must_use]
    pub fn from_env(env_var: &str) -> Self {
        Self {
            path:   parse_shot_path(env::var(env_var).ok().as_deref()),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

    /// Capture to an explicit path.
    #[must_use]
    pub fn with_path(path: CapturePath) -> Self {
        Self {
            path:   Some(path),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

    /// Override settle frame count.
    #[must_use]
    pub const fn settle(mut self, settle: SettleFrames) -> Self {
        self.settle = settle;
        self
    }

    /// Override post-capture poll budget.
    #[must_use]
    pub const fn poll_cap(mut self, poll: PollCap) -> Self {
        self.poll = poll;
        self
    }

    /// True when a capture path is set.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.path.is_some()
    }
}

impl Plugin for ScreenshotCapturePlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self.path.clone() else {
            return;
        };
        info!("gdtf_screenshot: capture ON -> {}", path.display());
        app.insert_resource(path)
            .insert_resource(self.settle)
            .insert_resource(self.poll)
            .init_resource::<CaptureProgress>()
            .add_systems(
                Update,
                (settle_then_capture, poll_then_exit)
                    .chain()
                    .run_if(resource_exists::<CapturePath>),
            );
    }
}
