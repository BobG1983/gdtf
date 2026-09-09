use serde::Deserialize;

use crate::mcp_shared::{mirror::ModeRow, save_fault::SaveFaultRow};

/// A client's own reading of why an editor write turned a mode down.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum RefusalRow {
    NoNewAction,
    ThemeNewIsUndoneBySync,
    NameBelongsToPrefabOnly,
    PrefabNeedsAName,
}

/// `editor.set_mode`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetModeReplyRow {
    pub(crate) mode: ModeRow,
}

/// What `editor.new` did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum NewOutcomeRow {
    Blanked,
    Refused(RefusalRow),
}

/// `editor.new`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct NewReplyRow {
    pub(crate) outcome: NewOutcomeRow,
}

/// What a per-tab load did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum LoadOutcomeRow {
    Loaded { key: String },
    NoSuchKey { key: String, known: Vec<String> },
}

/// A per-tab load's reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct LoadReplyRow {
    pub(crate) outcome: LoadOutcomeRow,
}

/// What `editor.save` did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum SaveOutcomeRow {
    Wrote { path: String },
    Failed(SaveFaultRow),
    Refused(RefusalRow),
}

/// `editor.save`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SaveReplyRow {
    pub(crate) outcome: SaveOutcomeRow,
}

/// What one recorded save did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum LastSaveOutcomeRow {
    Wrote { path: String },
    Failed(SaveFaultRow),
}

/// One row of `editor.last_save`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct LastSaveRow {
    pub(crate) mode:    ModeRow,
    pub(crate) outcome: LastSaveOutcomeRow,
}

/// `editor.last_save`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct LastSaveReplyRow {
    pub(crate) records: Vec<LastSaveRow>,
}
