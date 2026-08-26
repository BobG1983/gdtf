//! Editor mode tabs on the wire.

use serde::{Deserialize, Serialize};

use crate::EditorMode;

/// Which authoring workflow the editor's tab bar has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorModeNet {
    /// Terrain def form.
    Terrain,
    /// Theme def form.
    Theme,
    /// Prefab / map canvas.
    Prefab,
    /// Gang roster form.
    Gang,
    /// Armor form.
    Armor,
    /// Injury form.
    Injury,
    /// Sprite form.
    Sprite,
    /// Attachment form.
    Attachment,
    /// Ranged weapon form.
    Weapon,
    /// Melee weapon form.
    MeleeWeapon,
    /// Field def form.
    Field,
}

impl EditorModeNet {
    /// Every mode that owns a content registry, in the order a families read answers them.
    /// The Prefab tab is the map canvas, so it is not one of them.
    pub(in crate::net_qa) const CONTENT_FAMILIES: [Self; 10] = [
        Self::Terrain,
        Self::Theme,
        Self::Gang,
        Self::Armor,
        Self::Injury,
        Self::Sprite,
        Self::Attachment,
        Self::Weapon,
        Self::MeleeWeapon,
        Self::Field,
    ];

    /// Mirror the editor's own mode, with no wildcard arm.
    pub(in crate::net_qa) const fn from_mode(mode: EditorMode) -> Self {
        match mode {
            EditorMode::Terrain => Self::Terrain,
            EditorMode::Theme => Self::Theme,
            EditorMode::Prefab => Self::Prefab,
            EditorMode::Gang => Self::Gang,
            EditorMode::Armor => Self::Armor,
            EditorMode::Injury => Self::Injury,
            EditorMode::Sprite => Self::Sprite,
            EditorMode::Attachment => Self::Attachment,
            EditorMode::Weapon => Self::Weapon,
            EditorMode::MeleeWeapon => Self::MeleeWeapon,
            EditorMode::Field => Self::Field,
        }
    }

    /// Read a client's mode back as the editor's own, with no wildcard arm.
    pub(in crate::net_qa) const fn to_mode(self) -> EditorMode {
        match self {
            Self::Terrain => EditorMode::Terrain,
            Self::Theme => EditorMode::Theme,
            Self::Prefab => EditorMode::Prefab,
            Self::Gang => EditorMode::Gang,
            Self::Armor => EditorMode::Armor,
            Self::Injury => EditorMode::Injury,
            Self::Sprite => EditorMode::Sprite,
            Self::Attachment => EditorMode::Attachment,
            Self::Weapon => EditorMode::Weapon,
            Self::MeleeWeapon => EditorMode::MeleeWeapon,
            Self::Field => EditorMode::Field,
        }
    }
}
