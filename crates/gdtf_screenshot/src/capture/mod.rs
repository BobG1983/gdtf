//! One capture pipeline: settle, check the aim, spawn, verify, complete.

mod aim;
mod dir;
mod frames;
mod outcome;
mod plugin;
mod pump;
mod queue;
mod source;
mod spawn;
mod stem;
mod verify;

#[cfg(test)]
mod test;

pub use aim::{CaptureAim, CaptureAimDetail};
pub use dir::ShotDir;
pub use outcome::CaptureOutcome;
pub use plugin::{CapturePipelinePlugin, CaptureSystems};
pub use queue::{CaptureCompletion, CaptureCompletions, CaptureQueue};
pub use source::{CaptureSource, aims_at};
pub use stem::ShotStem;
