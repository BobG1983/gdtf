use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{phase_rows::ModeRow, support::TestError};

/// A client's own reading of why an editor write turned a mode down.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum RefusalRow {
    NoNewAction,
    ThemeNewIsUndoneBySync,
    NoLoadAction,
    NameBelongsToPrefabOnly,
    PrefabNeedsAName,
}

/// A painted cell as an illegal-cell fault names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct CellRow {
    pub(crate) x:     i32,
    pub(crate) y:     i32,
    pub(crate) level: u8,
}

/// A client's own reading of why a save wrote no file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum SaveFaultRow {
    EmptyName,
    MissingMountedWeapon,
    NoTerrain,
    DefaultFloorNotInTerrain,
    IllegalCell(CellRow),
    NoWorkspaceRoot,
    Serialize(String),
    Write(String),
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

/// What `editor.load` did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum LoadOutcomeRow {
    Loaded { key: String },
    NoSuchKey { key: String, known: Vec<String> },
    Refused(RefusalRow),
}

/// `editor.load`'s reply body.
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

/// Decode the RON body of a `Ran` outcome, or say which outcome came back instead.
pub(crate) fn ran_body<T: DeserializeOwned>(
    reply: &QaResponse,
    command: &str,
) -> Result<T, TestError> {
    let QaResponse::Outcome(CommandOutcome::Ran { reply: body, .. }) = reply else {
        return Err(format!("expected a Ran outcome for `{command}`, got {reply:?}").into());
    };
    Ok(ron::de::from_str::<T>(body.as_str())?)
}
