//! This crate is DEV / DEBUG infrastructure. It carries no gate of its own — the consuming crate
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
