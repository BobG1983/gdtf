//! One mode's newest save outcome on the wire.

use serde::{Deserialize, Serialize};

use super::{key::SavedPathNet, mode::EditorModeNet, save_fault::EditorSaveFaultNet};
use crate::save_record::SaveOutcome;

/// Where one mode's newest save landed, or why it landed nowhere.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum LastSaveOutcomeNet {
    /// The save wrote this file.
    Wrote {
        /// The file the save wrote.
        path: SavedPathNet,
    },
    /// The save wrote nothing, for this reason.
    Failed(EditorSaveFaultNet),
}

impl LastSaveOutcomeNet {
    /// Mirror the editor's own recorded outcome.
    #[must_use]
    pub(in crate::mcp) fn from_outcome(outcome: &SaveOutcome) -> Self {
        match outcome {
            SaveOutcome::Wrote(path) => Self::Wrote {
                path: SavedPathNet::from_path(path),
            },
            SaveOutcome::Failed(fault) => Self::Failed(EditorSaveFaultNet::from_fault(fault)),
        }
    }
}

/// One row of `editor.last_save`: the mode and what its newest save did.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLastSaveRowNet {
    /// The mode the save belongs to.
    mode:    EditorModeNet,
    /// What that save did.
    outcome: LastSaveOutcomeNet,
}

impl EditorLastSaveRowNet {
    /// Build a row for one mode's recorded outcome.
    #[must_use]
    pub(in crate::mcp) const fn new(mode: EditorModeNet, outcome: LastSaveOutcomeNet) -> Self {
        Self { mode, outcome }
    }
}
