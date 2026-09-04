//! The active mode's draft as the RON text a save would write, or why there is none.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::save_fault::EditorSaveFaultNet;

/// The RON text the save path would write for the current draft.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct EditorDraftRonNet(String);

impl EditorDraftRonNet {
    /// Wrap the serialized text.
    #[must_use]
    pub(in crate::mcp) const fn new(ron: String) -> Self {
        Self(ron)
    }
}

/// What projecting the active draft produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum EditorDraftOutcomeNet {
    /// The draft converted, and this is the file text a save would write.
    Ron(EditorDraftRonNet),
    /// The conversion refused this draft, so a save would write nothing either.
    NotSavable(EditorSaveFaultNet),
}
