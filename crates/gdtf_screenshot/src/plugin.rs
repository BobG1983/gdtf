use std::env;

use bevy::prelude::*;

use crate::{
    path::{CapturePath, parse_shot_path},
    settle::{PollCap, SettleFrames},
    trigger::{CaptureProgress, poll_then_exit, settle_then_capture},
};

pub struct ScreenshotCapturePlugin {
            path:   Option<CapturePath>,
        settle: SettleFrames,
        poll:   PollCap,
}

impl ScreenshotCapturePlugin {
                                #[must_use]
    pub fn from_env(env_var: &str) -> Self {
        Self {
            path:   parse_shot_path(env::var(env_var).ok().as_deref()),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

                #[must_use]
    pub fn with_path(path: CapturePath) -> Self {
        Self {
            path:   Some(path),
            settle: SettleFrames::default(),
            poll:   PollCap::default(),
        }
    }

            #[must_use]
    pub const fn settle(mut self, settle: SettleFrames) -> Self {
        self.settle = settle;
        self
    }

        #[must_use]
    pub const fn poll_cap(mut self, poll: PollCap) -> Self {
        self.poll = poll;
        self
    }

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
