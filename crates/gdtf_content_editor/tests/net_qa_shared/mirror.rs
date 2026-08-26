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
    Field,
}
