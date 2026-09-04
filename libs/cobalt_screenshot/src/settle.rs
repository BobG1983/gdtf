//! Frame budgets used before and after a capture.

use bevy::prelude::*;

/// How many frames to wait before taking the shot (UI / world settle).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct SettleFrames(u32);

impl SettleFrames {
    /// Default settle.
    pub const DEFAULT: Self = Self(30);

    /// Explicit frame count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

impl Default for SettleFrames {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Max frames to wait for the PNG to appear on disk before giving up.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct PollCap(u32);

impl PollCap {
    /// Default poll budget.
    pub const DEFAULT: Self = Self(600);

    /// Explicit poll frame limit.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

impl Default for PollCap {
    fn default() -> Self {
        Self::DEFAULT
    }
}
