//! One capture pipeline: settle, spawn the readback, verify, complete.

mod dir;
mod frames;
mod outcome;
mod plugin;
mod pump;
mod queue;
mod spawn;
mod stem;
mod verify;

#[cfg(test)]
mod test;

pub use dir::{ShotDir, ShotDirName};
pub use outcome::CaptureOutcome;
pub use plugin::{CapturePipelinePlugin, CaptureSystems};
pub use queue::{CaptureCompletion, CaptureCompletions, CaptureQueue};
pub use stem::ShotStem;
