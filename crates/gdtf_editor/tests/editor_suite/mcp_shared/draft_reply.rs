use serde::Deserialize;

use crate::mcp_shared::{mirror::ModeRow, save_fault::SaveFaultRow};

/// What projecting the active draft produced.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum DraftOutcomeRow {
    Ron(String),
    NotSavable(SaveFaultRow),
}

/// `editor.draft`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct DraftReplyRow {
    pub(crate) mode:    ModeRow,
    pub(crate) outcome: DraftOutcomeRow,
}
