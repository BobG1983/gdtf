//! The content families whose registries the editor's validation pass watches.

use serde::{Deserialize, Serialize};

/// One content family that owns a registry a `wait` can watch for a rearm.
///
/// Each variant is spelled the way [`super::mode::EditorModeNet`] spells the same tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum ContentFamilyNet {
    /// The ranged weapon registry.
    Weapon,
    /// The melee weapon registry.
    MeleeWeapon,
    /// The armor registry.
    Armor,
    /// The gang roster registry.
    Gang,
    /// The terrain def registry.
    Terrain,
    /// The theme registry.
    Theme,
    /// The injury registry.
    Injury,
    /// The sprite def registry.
    Sprite,
    /// The attachment registry.
    Attachment,
    /// The field def registry.
    Field,
}

impl ContentFamilyNet {
    /// Every family a wait can name, in the order `WatchedRegistries` holds them.
    pub(in crate::net_qa) const ALL: [Self; 10] = [
        Self::Weapon,
        Self::MeleeWeapon,
        Self::Armor,
        Self::Gang,
        Self::Terrain,
        Self::Theme,
        Self::Injury,
        Self::Sprite,
        Self::Attachment,
        Self::Field,
    ];
}
