//! What the editor's last save per mode did, and the faults a save can report.

mod fault;
mod record;

pub use fault::{EditorSaveFault, SaveFaultMessage};
pub use record::{LastSaveRecord, SaveOutcome, SavedAssetPath};
