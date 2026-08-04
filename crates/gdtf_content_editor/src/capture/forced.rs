use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::{equipment::attachments::AttachmentName, weapon::WeaponName};

use crate::{EditorMode, canvas::CanvasZoom, terrain_form::TerrainKindChoice};

#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedMode(EditorMode);

impl ForcedMode {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "terrain" => Some(Self(EditorMode::Terrain)),
            "theme" => Some(Self(EditorMode::Theme)),
            "prefab" => Some(Self(EditorMode::Prefab)),
            "gang" => Some(Self(EditorMode::Gang)),
            "armor" => Some(Self(EditorMode::Armor)),
            "injury" => Some(Self(EditorMode::Injury)),
            "sprite" => Some(Self(EditorMode::Sprite)),
            "attachment" => Some(Self(EditorMode::Attachment)),
            "weapon" => Some(Self(EditorMode::Weapon)),
            "melee_weapon" => Some(Self(EditorMode::MeleeWeapon)),
            _ => None,
        }
    }
}

#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedTerrainKind(TerrainKindChoice);

impl ForcedTerrainKind {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wall" => Some(Self(TerrainKindChoice::Wall)),
            "cover" => Some(Self(TerrainKindChoice::Cover)),
            "slab" => Some(Self(TerrainKindChoice::Slab)),
            "emplacement" => Some(Self(TerrainKindChoice::Emplacement)),
            _ => None,
        }
    }
}

#[derive(Resource, Clone, Deref)]
pub(super) struct ForcedAttachment(AttachmentName);

impl ForcedAttachment {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        let key = value.trim();
        (!key.is_empty()).then(|| Self(AttachmentName::new(key.to_owned())))
    }
}

#[derive(Resource, Clone, Deref)]
pub(super) struct ForcedWeapon(WeaponName);

impl ForcedWeapon {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        let key = value.trim();
        (!key.is_empty()).then(|| Self(WeaponName::new(key.to_owned())))
    }
}

#[derive(Resource, Clone, Deref)]
pub(super) struct ForcedMeleeWeapon(WeaponName);

impl ForcedMeleeWeapon {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        let key = value.trim();
        (!key.is_empty()).then(|| Self(WeaponName::new(key.to_owned())))
    }
}

#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedZoom(CanvasZoom);

impl ForcedZoom {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        let factor = value.trim().parse::<f32>().ok().filter(|f| f.is_finite())?;
        Some(Self(CanvasZoom::identity().scaled(factor)))
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub(super) enum ForcedView {
    Full,
    Isolate,
}

impl ForcedView {
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "full" | "fullview" | "full_view" => Some(Self::Full),
            "isolate" => Some(Self::Isolate),
            _ => None,
        }
    }
}
