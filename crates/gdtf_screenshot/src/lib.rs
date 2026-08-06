//! Dev/debug screenshot helpers for Bevy apps.

pub mod capture;
pub mod path;
pub mod present;
pub mod settle;

pub use capture::{
    CaptureAim, CaptureAimDetail, CaptureCompletion, CaptureCompletions, CaptureOutcome,
    CapturePipelinePlugin, CaptureQueue, CaptureSource, CaptureSystems, ShotDir, ShotStem, aims_at,
};
pub use path::CapturePath;
pub use present::{
    CapturePresentPlugin, PRESENT_LAYER, PRESENT_ORDER, PresentCamera, PresentSprite,
    PresentSystems, QaCaptureTarget,
};
pub use settle::{PollCap, SettleFrames};
