use serde::Deserialize;

use crate::{mirror::ModeRow, save_fault::SaveFaultRow};

/// A client's own reading of why an editor write turned a mode down.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum RefusalRow {
    NoNewAction,
    ThemeNewIsUndoneBySync,
    NoLoadAction,
    NameBelongsToPrefabOnly,
    PrefabNeedsAName,
}

/// `editor.set_mode`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetModeReplyRow {
    pub(crate) mode: ModeRow,
}

/// A client's own reading of the Terrain draft's kind pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum TerrainKindRow {
    Wall,
    Cover,
    Slab,
    Emplacement,
}

/// A client's own reading of a cardinal side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FacingRow {
    North,
    East,
    South,
    West,
}

/// Which single-value field a write named, carrying the value read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FieldRow {
    Kind(TerrainKindRow),
}

/// Which list a write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    EntrySides,
}

/// `editor.set_field`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetFieldReplyRow {
    pub(crate) field: FieldRow,
}

/// `editor.list_op`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct ListOpReplyRow {
    pub(crate) list:    ListRow,
    pub(crate) members: Vec<FacingRow>,
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
