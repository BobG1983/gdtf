//! Editor screenshot settle, poll budget, and capture source.

use bevy::{camera::ImageRenderTarget, image::Image, prelude::*};
use gdtf_screenshot::{PollCap, SettleFrames};

/// Frames to wait before capturing an editor shot.
#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotSettle(SettleFrames);

impl EditorShotSettle {
    /// Build from a settle frame count.
    #[must_use]
    pub const fn new(frames: SettleFrames) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotSettle {
    fn default() -> Self {
        Self(SettleFrames::DEFAULT_EGUI)
    }
}

/// Max frames to poll for a finished editor shot.
#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotPollBudget(PollCap);

impl EditorShotPollBudget {
    /// Build from a poll frame budget.
    #[must_use]
    pub const fn new(frames: PollCap) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotPollBudget {
    fn default() -> Self {
        Self(PollCap::DEFAULT)
    }
}

/// Where the editor shot is captured from.
#[derive(Resource, Clone, Debug)]
pub enum EditorShotSource {
    /// Capture the primary window.
    PrimaryWindow,
    /// Capture an offscreen image target.
    Offscreen(ImageRenderTarget),
}

impl Default for EditorShotSource {
    fn default() -> Self {
        Self::Offscreen(ImageRenderTarget::from(Handle::<Image>::default()))
    }
}
