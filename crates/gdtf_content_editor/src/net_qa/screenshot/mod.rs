mod aim;
mod config;
mod path;
mod payload;
mod pump;
mod spawn;
mod verify;

#[cfg(test)]
mod test;

pub use config::{EditorShotPollBudget, EditorShotSettle, EditorShotSource};
pub use path::EditorQaShotDir;
pub(super) use path::EditorShotSequence;
pub use payload::EditorScreenshotPayload;
pub(super) use pump::{EditorInFlightShots, drive_editor_screenshots};
