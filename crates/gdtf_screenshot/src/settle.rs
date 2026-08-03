//! Frame budgets used before and after a capture.

use bevy::prelude::*;

/// How many frames to wait before taking the shot (UI / world settle).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct SettleFrames(u32);

impl SettleFrames {
    /// Default settle for egui-heavy scenes.
    pub const DEFAULT_EGUI: Self = Self(30);

    /// Default settle for battle scenes.
    pub const DEFAULT_BATTLE: Self = Self(15);

    /// Explicit frame count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

impl Default for SettleFrames {
    fn default() -> Self {
        Self::DEFAULT_EGUI
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

#[cfg(test)]
mod tests {
    use super::{PollCap, SettleFrames};

    #[test]
    fn settle_defaults_are_the_calibrated_values() {
        assert_eq!(*SettleFrames::DEFAULT_EGUI, 30);
        assert_eq!(*SettleFrames::DEFAULT_BATTLE, 15);
        assert_eq!(SettleFrames::default(), SettleFrames::DEFAULT_EGUI);
    }

    #[test]
    fn poll_cap_default_is_calibrated() {
        assert_eq!(*PollCap::DEFAULT, 600);
        assert_eq!(PollCap::default(), PollCap::DEFAULT);
    }

    #[test]
    fn new_wraps_explicit_counts() {
        assert_eq!(*SettleFrames::new(45), 45);
        assert_eq!(*PollCap::new(120), 120);
    }
}
