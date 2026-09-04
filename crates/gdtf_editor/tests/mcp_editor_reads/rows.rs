use serde::Deserialize;

/// `editor.validation`'s reply body as a client decodes it.
#[derive(Debug, Deserialize)]
pub(crate) struct ValidationReplyRow {
    pub(crate) checks_complete: bool,
    pub(crate) published:       bool,
    pub(crate) findings:        Vec<String>,
}

/// A client's own reading of which registry a family row names.
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

/// One registry member: the key a load takes, and the label a picker shows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct FamilyEntryRow {
    pub(crate) key:   String,
    pub(crate) label: String,
}

/// One family and every member it holds.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct FamilyRowEntries {
    pub(crate) family:  FamilyRow,
    pub(crate) entries: Vec<FamilyEntryRow>,
}

/// `editor.families`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct FamiliesReplyRow {
    pub(crate) families: Vec<FamilyRowEntries>,
}

/// A client's own reading of the prefab grid's extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct GridSizeRow {
    pub(crate) width:  u8,
    pub(crate) height: u8,
    pub(crate) levels: u8,
}

/// A client's own reading of which storeys the canvas draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ViewModeRow {
    DownToActive,
    FullView,
}

/// A client's own reading of the onion isolation around the active storey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum IsolateRow {
    Off,
    On(u8),
}

/// A client's own reading of the preview camera's pan offset.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct PanRow {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// A client's own reading of what the canvas draws and from where.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct ViewRow {
    pub(crate) mode:    ViewModeRow,
    pub(crate) isolate: IsolateRow,
    pub(crate) zoom:    f32,
    pub(crate) pan:     PanRow,
}

/// A client's own reading of a cardinal side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FacingRow {
    North,
    East,
    South,
    West,
}

/// `editor.session`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SessionReplyRow {
    pub(crate) theme:         String,
    pub(crate) default_floor: Option<String>,
    pub(crate) grid_size:     GridSizeRow,
    pub(crate) selected_tile: Option<String>,
    pub(crate) facing:        FacingRow,
    pub(crate) level:         u8,
    pub(crate) view:          ViewRow,
}
