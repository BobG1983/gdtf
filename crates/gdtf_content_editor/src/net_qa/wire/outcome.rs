//! What a blank, a load, or a save did, as a typed outcome inside a successful reply.

use serde::{Deserialize, Serialize};

use super::{
    key::{EditorKeyNet, SavedPathNet},
    refusal::EditorRefusalNet,
    save_fault::EditorSaveFaultNet,
};
use crate::save_record::SaveOutcome;

/// What `editor.new` did to the named mode's draft.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorNewOutcomeNet {
    /// The draft was replaced with that form's blank-draft constructor.
    Blanked,
    /// This build blanks no draft for that mode.
    Refused(EditorRefusalNet),
}

/// What `editor.load` did to the named mode's draft.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorLoadOutcomeNet {
    /// The registry entry was loaded into the draft.
    Loaded {
        /// The key that was loaded.
        key: EditorKeyNet,
    },
    /// The registry holds no entry under that key; the draft is untouched.
    NoSuchKey {
        /// The key that was asked for.
        key:   EditorKeyNet,
        /// Every key that registry does hold.
        known: Vec<EditorKeyNet>,
    },
    /// This build loads no draft for that mode.
    Refused(EditorRefusalNet),
}

/// What `editor.save` wrote, or why it wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorSaveOutcomeNet {
    /// The writer wrote this file.
    Wrote {
        /// The file the save wrote.
        path: SavedPathNet,
    },
    /// The writer ran and reported this fault.
    Failed(EditorSaveFaultNet),
    /// The arguments named a shape this build does not save.
    Refused(EditorRefusalNet),
}

impl EditorSaveOutcomeNet {
    /// Mirror what a writer that ran reported, which is never a refusal.
    #[must_use]
    pub(in crate::net_qa) fn from_outcome(outcome: &SaveOutcome) -> Self {
        match outcome {
            SaveOutcome::Wrote(path) => Self::Wrote {
                path: SavedPathNet::from_path(path),
            },
            SaveOutcome::Failed(fault) => Self::Failed(EditorSaveFaultNet::from_fault(fault)),
        }
    }
}
