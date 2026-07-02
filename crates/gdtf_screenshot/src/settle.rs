//! Settle-timing newtypes: how many frames to wait before capturing, and how long to poll for the
//! written PNG before giving up.
//!
//! A screenshot must be taken AFTER the UI has laid out and drawn (settle-before-read), or the PNG
//! reflects a mid-layout / pre-composite frame. Different scenes settle at different rates (egui +
//! an offscreen render-to-texture pass needs more frames than a simple loading screen), so the
//! settle window is PARAMETERISED as [`SettleFrames`] with documented defaults. [`PollCap`] bounds
//! the disk-poll that waits for the async GPU readback to flush the PNG, so a failed write can never
//! hang a capture-then-exit run.

use bevy::prelude::*;

/// How many frames to wait — counted from when the capture becomes eligible — before spawning the
/// [`Screenshot`](bevy::render::view::window::screenshot::Screenshot) request.
///
/// A named newtype over `u32` (no-bare-types): the settle window is a domain value the plugin
/// threads through as a [`Resource`]. The inner is PRIVATE — construct via [`SettleFrames::new`] or
/// a documented default, read through [`Deref`].
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct SettleFrames(u32);

impl SettleFrames {
    /// The safe default for an egui / `bevy_ui` scene with an offscreen render-to-texture pass: 30
    /// frames. Matches the calibrated editor-capture settle (`gdtf_content_editor`), which needs a
    /// couple of extra frames beyond the egui layout for the first offscreen composite.
    pub const DEFAULT_EGUI: Self = Self(30);

    /// The default for a simple, single-pass scene (a battlescape HUD): 15 frames — enough for
    /// Bevy's UI layout to flush without waiting on an offscreen pass. Matches the calibrated
    /// battlescape-capture settle (`gdtf_app`).
    pub const DEFAULT_BATTLE: Self = Self(15);

    /// Wrap an explicit settle-frame count (a caller that has calibrated its own scene's settle,
    /// e.g. the procgen visualizer's 45-frame reveal-sync window).
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

impl Default for SettleFrames {
    /// The wiring default: [`SettleFrames::DEFAULT_EGUI`] — the safe choice for the egui editor and
    /// any `bevy_ui` scene (a caller with a faster scene can pass [`SettleFrames::DEFAULT_BATTLE`]).
    fn default() -> Self {
        Self::DEFAULT_EGUI
    }
}

/// How many frames to poll the disk for the written PNG (after the screenshot is requested) before
/// giving up and exiting anyway — a safety cap so a failed write never hangs a capture-then-exit
/// run.
///
/// A named newtype over `u32` (no-bare-types). The inner is PRIVATE — construct via
/// [`PollCap::new`] or [`PollCap::DEFAULT`], read through [`Deref`].
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct PollCap(u32);

impl PollCap {
    /// The default poll cap: 600 frames (~10 s at 60 fps) — matches the calibrated editor-capture
    /// cap. Long enough that a slow GPU readback still lands the PNG, short enough that a genuinely
    /// failed write does not hang the process.
    pub const DEFAULT: Self = Self(600);

    /// Wrap an explicit poll cap.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

impl Default for PollCap {
    /// The wiring default: [`PollCap::DEFAULT`].
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
