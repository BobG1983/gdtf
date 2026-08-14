//! What an editor command's availability check may read about the host.

use crate::net_qa::wire::{EditorModeNet, EditorPhaseNet};

/// One frame's reading of the editor's lifecycle phase and open mode tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::net_qa) struct EditorFacts {
    phase: EditorPhaseNet,
    mode:  Option<EditorModeNet>,
}

impl EditorFacts {
    /// Build the facts for one frame.
    pub(in crate::net_qa) const fn new(phase: EditorPhaseNet, mode: Option<EditorModeNet>) -> Self {
        Self { phase, mode }
    }

    /// Where the editor is in its lifecycle.
    #[must_use]
    pub(in crate::net_qa) const fn phase(self) -> EditorPhaseNet {
        self.phase
    }

    /// The open mode tab, absent until the authoring scene is live.
    #[must_use]
    pub(in crate::net_qa) const fn mode(self) -> Option<EditorModeNet> {
        self.mode
    }
}
