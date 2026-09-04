use serde::Deserialize;

use crate::{mirror::ModeRow, save_fault::SaveFaultRow};

/// `editor.set_mode`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetModeReplyRow {
    pub(crate) mode: ModeRow,
}

/// A client's own reading of why a write command turned a mode down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum RefusalRow {
    NoNewAction,
    ThemeNewIsUndoneBySync,
    NameBelongsToPrefabOnly,
    PrefabNeedsAName,
}

/// What `editor.new` did to the named mode's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum NewOutcomeRow {
    Blanked,
    Refused(RefusalRow),
}

/// `editor.new`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct NewReplyRow {
    pub(crate) outcome: NewOutcomeRow,
}

/// A client's own reading of a sheet rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct RectRow {
    pub(crate) w: u32,
    pub(crate) h: u32,
}

/// A client's own reading of where a sprite's pixels come from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum SourceRow {
    File(String),
    Sheet { sheet: String, rect: RectRow },
}

/// A client's own reading of a sprite facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FacingRow {
    North,
    East,
    South,
    West,
}

/// A Sprite field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum SpriteFieldRow {
    Name(String),
    BaseSource(SourceRow),
    AnchorX(u32),
    AnchorY(u32),
    Fps(f32),
    FacingOverride {
        facing: FacingRow,
        source: Option<SourceRow>,
    },
    Frame {
        index:  usize,
        source: SourceRow,
    },
    Animated(bool),
}

/// Which single-value field a write named, under the form that owns it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum FieldRow {
    Sprite(SpriteFieldRow),
}

/// `editor.set_field`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetFieldReplyRow {
    pub(crate) mode:  ModeRow,
    pub(crate) field: FieldRow,
}

/// Which list a write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    SpriteFrames,
}

/// One member of the list a reply reads back.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum ListMemberRow {
    SpriteFrame(SourceRow),
}

/// `editor.list_op`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct ListOpReplyRow {
    pub(crate) mode:    ModeRow,
    pub(crate) list:    ListRow,
    pub(crate) members: Vec<ListMemberRow>,
}

/// What `editor.save` wrote, or why it wrote nothing.
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

/// What the newest save under one mode did, as `editor.last_save` reports it.
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
