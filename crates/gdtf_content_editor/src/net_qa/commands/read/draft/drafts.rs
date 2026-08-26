//! The ten form drafts a read projects, borrowed read-only.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    armor_form::ArmorDraft, attachment_form::AttachmentDraft, field_form::FieldDraft,
    gang_form::GangDraft, injury_form::InjuryDraft, melee_weapon_form::MeleeWeaponDraft,
    sprite_form::SpriteDraft, terrain_form::TerrainDraft, theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

/// Every draft `editor.draft` can project, all scoped to Editing so all `Option`.
#[derive(SystemParam)]
pub(super) struct FormDrafts<'w> {
    pub(super) terrain:      Option<Res<'w, TerrainDraft>>,
    pub(super) theme:        Option<Res<'w, ThemeDraft>>,
    pub(super) gang:         Option<Res<'w, GangDraft>>,
    pub(super) armor:        Option<Res<'w, ArmorDraft>>,
    pub(super) injury:       Option<Res<'w, InjuryDraft>>,
    pub(super) sprite:       Option<Res<'w, SpriteDraft>>,
    pub(super) attachment:   Option<Res<'w, AttachmentDraft>>,
    pub(super) weapon:       Option<Res<'w, WeaponDraft>>,
    pub(super) melee_weapon: Option<Res<'w, MeleeWeaponDraft>>,
    pub(super) field:        Option<Res<'w, FieldDraft>>,
}
