//! Editor lifecycle phase on the wire.

use serde::{Deserialize, Serialize};

use crate::EditorState;

/// Where the editor is in its load-then-author lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum EditorPhaseNet {
    /// Asset / registry load in progress.
    Load,
    /// Authoring scene is live.
    Editing,
}

impl EditorPhaseNet {
    /// Mirror the editor's own state, with no wildcard arm.
    pub(in crate::mcp) const fn from_state(state: &EditorState) -> Self {
        match state {
            EditorState::Load => Self::Load,
            EditorState::Editing => Self::Editing,
        }
    }
}
