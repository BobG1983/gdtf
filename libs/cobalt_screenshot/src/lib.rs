//! Dev/debug screenshot helpers for Bevy apps.

pub mod capture;
pub mod path;
pub mod present;
pub mod settle;
pub mod window_capture;

pub use capture::{
    CaptureCompletion, CaptureCompletions, CaptureOutcome, CapturePipelinePlugin, CaptureQueue,
    CaptureSystems, ShotDir, ShotDirName, ShotStem,
};
pub use path::CapturePath;
pub use present::CapturePresentPlugin;
pub use settle::{PollCap, SettleFrames};
pub use window_capture::{CaptureImage, WindowCapturePlugin};
