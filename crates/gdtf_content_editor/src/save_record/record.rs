//! The latest save outcome the editor recorded for each mode.

use std::path::PathBuf;

use bevy::{platform::collections::HashMap, prelude::*};

use super::fault::EditorSaveFault;
use crate::mode::EditorMode;

/// The file a save wrote.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct SavedAssetPath(PathBuf);

impl SavedAssetPath {
    /// Wrap the path a save wrote.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// Where a save landed, or why it landed nowhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The save wrote this file.
    Wrote(SavedAssetPath),
    /// The save wrote nothing, for this reason.
    Failed(EditorSaveFault),
}

impl SaveOutcome {
    /// Read a writer's result into an outcome, converting the fault through `From`.
    #[must_use]
    pub fn from_result<E: Into<EditorSaveFault>>(result: Result<PathBuf, E>) -> Self {
        match result {
            Ok(path) => Self::Wrote(SavedAssetPath::new(path)),
            Err(err) => Self::Failed(err.into()),
        }
    }
}

/// The latest save each mode reported, whichever path drove it.
#[derive(Resource, Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct LastSaveRecord(HashMap<EditorMode, SaveOutcome>);

impl LastSaveRecord {
    /// Record what the newest save for `mode` did, replacing the one before it.
    pub fn record(&mut self, mode: EditorMode, outcome: SaveOutcome) {
        self.0.insert(mode, outcome);
    }

    /// The newest save recorded for `mode`, if that mode has saved at all.
    #[must_use]
    pub fn outcome(&self, mode: EditorMode) -> Option<&SaveOutcome> {
        self.0.get(&mode)
    }
}
