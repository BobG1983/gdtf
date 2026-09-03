use serde::Deserialize;

use crate::mirror::ModeRow;

/// `editor.set_mode`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetModeReplyRow {
    pub(crate) mode: ModeRow,
}

/// A client's own reading of the Injury tab's sub-tab, decoded from the wire by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SubTabRow {
    Def,
    Tables,
}

/// `editor.select_injury_tab`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SelectInjuryTabReplyRow {
    pub(crate) tab: SubTabRow,
}

/// One registry member as `editor.families` answers it.
#[derive(Debug, Deserialize)]
pub(crate) struct FamilyEntryRow {
    pub(crate) key: String,
}

/// A client's own reading of which registry a families row names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FamilyRow {
    Terrain,
    Theme,
    Gang,
    Armor,
    Injury,
    Sprite,
    Attachment,
    Weapon,
    MeleeWeapon,
    Field,
}

/// One family and every member it holds.
#[derive(Debug, Deserialize)]
pub(crate) struct FamilyRowEntries {
    pub(crate) family:  FamilyRow,
    pub(crate) entries: Vec<FamilyEntryRow>,
}

/// `editor.families`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct FamiliesReplyRow {
    pub(crate) families: Vec<FamilyRowEntries>,
}

/// `editor.select_theme`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SelectThemeReplyRow {
    pub(crate) theme:         String,
    pub(crate) default_floor: Option<String>,
}

/// A client's own reading of which way a terrain toggle went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ToggleRow {
    Added,
    Removed,
}

/// `editor.toggle_terrain`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct ToggleTerrainReplyRow {
    pub(crate) toggle:        ToggleRow,
    pub(crate) default_floor: Option<String>,
}

/// `editor.set_default_floor`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetDefaultFloorReplyRow {
    pub(crate) default_floor: Option<String>,
}

/// A client's own reading of the lifecycle phase a reply was answered in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum PhaseRow {
    Load,
    Editing,
}

/// A client's own reading of the nine families a wait can watch. Prefab owns no registry and
/// `FieldDefRegistry` is not watched, so neither is spelled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ContentFamilyRow {
    Weapon,
    MeleeWeapon,
    Armor,
    Gang,
    Terrain,
    Theme,
    Injury,
    Sprite,
    Attachment,
}

/// A client's own reading of the condition a `wait` was holding out for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum WaitConditionRow {
    ChecksComplete,
    RegistryRearmed { family: ContentFamilyRow },
}

/// `wait`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct WaitReplyRow {
    pub(crate) condition: WaitConditionRow,
    pub(crate) phase:     PhaseRow,
}

/// A client's own reading of why a delete was refused.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum DeleteRefusalRow {
    NoEntry,
    NoRecord,
    InUse(Vec<ReferringRecordRow>),
}

/// One record the in-use check named as still referencing the deleted one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ReferringRecordRow {
    pub(crate) family: String,
    pub(crate) key:    String,
    pub(crate) field:  String,
}

/// `editor.delete_record`'s reply body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum DeleteOutcomeRow {
    Refused(DeleteRefusalRow),
    Removed,
}
