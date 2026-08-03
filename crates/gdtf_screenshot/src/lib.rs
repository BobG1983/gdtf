//! Dev/debug screenshot helpers for Bevy apps.
//!
//! No product gate of its own — the consuming crate decides when this is linked.
//! Supports keyboard capture (F10) and env/path-driven settle-then-exit capture.

pub mod keybind;
pub mod path;
pub mod plugin;
pub mod settle;
pub mod trigger;

pub use keybind::{CaptureTag, KeyboardCapturePlugin};
pub use path::{CapturePath, parse_shot_path, timestamped_path};
pub use plugin::ScreenshotCapturePlugin;
pub use settle::{PollCap, SettleFrames};
pub use trigger::{
    CaptureProgress, FrameCount, ShotRequested, poll_then_exit, reset_progress, settle_then_capture,
};
