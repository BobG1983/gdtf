use serde::Deserialize;

use crate::values::{ArmorTypeRow, BodyPartRow, EffectRow, FacingRow, SlotRow, SourceRow};

/// Which single-value field a write named, carrying the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum FieldRow {
    ArmorName(String),
    SpriteName(String),
    AttachmentName(String),
    ArmorFloor {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorProtection {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorHardness {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorIntegrity {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorType {
        part:  BodyPartRow,
        value: ArmorTypeRow,
    },
    SpriteBaseSource(SourceRow),
    SpriteAnchorX(u32),
    SpriteAnchorY(u32),
    SpriteFps(f32),
    SpriteFacingOverride {
        facing: FacingRow,
        source: Option<SourceRow>,
    },
    SpriteFrame {
        index:  usize,
        source: SourceRow,
    },
    SpriteAnimated(bool),
    AttachmentDisplayName(String),
    AttachmentSlot(SlotRow),
    AttachmentEffect {
        index:  usize,
        effect: EffectRow,
    },
}

/// Which list a write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    EntrySides,
    AttachmentEffects,
    SpriteFrames,
}

/// One member of the list a reply reads back.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ListMemberRow {
    EntrySide(FacingRow),
    AttachmentEffect(EffectRow),
    SpriteFrame(SourceRow),
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
    pub(crate) members: Vec<ListMemberRow>,
}
