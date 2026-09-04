//! What an editor command's availability check may read about the host.

use super::DraftInWorld;
use crate::mcp::wire::{EditorModeNet, EditorPhaseNet};

/// One frame's reading of the editor's lifecycle phase, open mode tab and that tab's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mcp) struct EditorFacts {
    phase: EditorPhaseNet,
    mode:  Option<EditorModeNet>,
    draft: DraftInWorld,
}

impl EditorFacts {
    /// Build the facts for one frame.
    pub(in crate::mcp) const fn new(
        phase: EditorPhaseNet,
        mode: Option<EditorModeNet>,
        draft: DraftInWorld,
    ) -> Self {
        Self { phase, mode, draft }
    }

    /// Where the editor is in its lifecycle.
    #[must_use]
    pub(in crate::mcp) const fn phase(self) -> EditorPhaseNet {
        self.phase
    }

    /// The open mode tab, absent until the authoring scene is live.
    #[must_use]
    pub(in crate::mcp) const fn mode(self) -> Option<EditorModeNet> {
        self.mode
    }

    /// Whether the open tab's own draft resource is in the world.
    #[must_use]
    pub(in crate::mcp) const fn draft(self) -> DraftInWorld {
        self.draft
    }
}
