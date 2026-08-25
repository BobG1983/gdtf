use serde::Deserialize;

use crate::mirror::ModeRow;

/// `editor.set_mode`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetModeReplyRow {
    pub(crate) mode: ModeRow,
}

/// One registry member as `editor.families` answers it.
#[derive(Debug, Deserialize)]
pub(crate) struct FamilyEntryRow {
    pub(crate) key: String,
}

/// One family and every member it holds.
#[derive(Debug, Deserialize)]
pub(crate) struct FamilyRowEntries {
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
