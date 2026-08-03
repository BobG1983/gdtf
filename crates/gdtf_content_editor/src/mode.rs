//! The editor is a multi-mode "Workbench": exactly one authoring workflow (one
use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorMode {
        Terrain,
            Theme,
        #[default]
    Prefab,
                Gang,
                Armor,
                    Injury,
                    Sprite,
                    Attachment,
                        Weapon,
                        MeleeWeapon,
}

impl EditorMode {
                            pub const TAB_ORDER: [Self; 10] = [
        Self::Terrain,
        Self::Theme,
        Self::Prefab,
        Self::Gang,
        Self::Armor,
        Self::Injury,
        Self::Sprite,
        Self::Attachment,
        Self::Weapon,
        Self::MeleeWeapon,
    ];

            #[must_use]
    pub fn from_tab_index(index: usize) -> Option<Self> {
        Self::TAB_ORDER.get(index).copied()
    }

        #[must_use]
    pub fn tab_index(self) -> usize {
        Self::TAB_ORDER
            .iter()
            .position(|mode| *mode == self)
            .unwrap_or(0)
    }

        #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Terrain => "TERRAIN",
            Self::Theme => "THEME",
            Self::Prefab => "PREFAB",
            Self::Gang => "GANG",
            Self::Armor => "ARMOR",
            Self::Injury => "INJURY",
            Self::Sprite => "SPRITE",
            Self::Attachment => "ATTACHMENT",
            Self::Weapon => "WEAPON",
            Self::MeleeWeapon => "MELEE",
        }
    }
}

pub(crate) fn mode_hotkeys(keys: Res<ButtonInput<KeyCode>>, mode: Option<ResMut<EditorMode>>) {
    let Some(mut mode) = mode else {
        return;
    };
    let pressed = if keys.just_pressed(KeyCode::Digit1) {
        Some(EditorMode::Terrain)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(EditorMode::Theme)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(EditorMode::Prefab)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(EditorMode::Gang)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(EditorMode::Armor)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(EditorMode::Injury)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(EditorMode::Sprite)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(EditorMode::Attachment)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(EditorMode::Weapon)
    } else if keys.just_pressed(KeyCode::Digit0) {
        Some(EditorMode::MeleeWeapon)
    } else {
        None
    };
    if let Some(next) = pressed {
        mode.set_if_neq(next);
    }
}
