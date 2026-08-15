//! The three modes `editor.new` will not blank, and the reason each carries.

use crate::{EditorMode, net_qa::wire::EditorRefusalNet};

/// Why this build blanks no draft for `mode`, or `None` when it does blank one.
pub(in crate::net_qa::commands::write::blank) const fn refusal_for(
    mode: EditorMode,
) -> Option<EditorRefusalNet> {
    match mode {
        EditorMode::Terrain | EditorMode::Prefab => Some(EditorRefusalNet::NoNewAction),
        EditorMode::Theme => Some(EditorRefusalNet::ThemeNewIsUndoneBySync),
        EditorMode::Gang
        | EditorMode::Armor
        | EditorMode::Injury
        | EditorMode::Sprite
        | EditorMode::Attachment
        | EditorMode::Weapon
        | EditorMode::MeleeWeapon => None,
    }
}
