use serde::Deserialize;

/// A client's own reading of the editor's mode tab, decoded from the wire by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ModeRow {
    Terrain,
    Theme,
    Prefab,
    Gang,
    Armor,
    Injury,
    Sprite,
    Attachment,
    Weapon,
    MeleeWeapon,
}

/// A painted cell as an illegal-cell fault names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct CellRow {
    pub(crate) x:     i32,
    pub(crate) y:     i32,
    pub(crate) level: u8,
}

/// A client's own reading of why a draft would write no file.
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
