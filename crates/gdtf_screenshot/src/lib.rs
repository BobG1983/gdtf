//! Dev/debug screenshot helpers for Bevy apps.

pub mod capture;
pub mod keybind;
pub mod path;
pub mod plugin;
pub mod present;
pub mod settle;

pub use capture::{
    CaptureAim, CaptureAimDetail, CaptureCompletion, CaptureCompletions, CaptureOutcome,
    CapturePipelinePlugin, CaptureQueue, CaptureSource, CaptureSystems, ShotDir, ShotStem, aims_at,
};
pub use keybind::{CaptureTag, KeyboardCapturePlugin};
pub use path::{CapturePath, parse_shot_path};
pub use plugin::ScreenshotCapturePlugin;
pub use present::{
    CapturePresentPlugin, PRESENT_LAYER, PRESENT_ORDER, PresentCamera, PresentSprite,
    PresentSystems, QaCaptureTarget,
};
pub use settle::{PollCap, SettleFrames};
