//! Every in-progress draft a QA write command may blank, load, or save.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    armor_form::ArmorDraft, attachment_form::AttachmentDraft, editor_map::EditorMap,
    gang_form::GangDraft, injury_form::InjuryDraft, melee_weapon_form::MeleeWeaponDraft,
    session::MapEditorSession, sprite_form::SpriteDraft, terrain_form::TerrainDraft,
    theme_form::ThemeDraft, weapon_form::WeaponDraft,
};

/// The authoring session and the ten drafts the editor's forms hold, all scoped to Editing.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorForms<'w> {
    pub(in crate::net_qa) session:      Option<ResMut<'w, MapEditorSession>>,
    pub(in crate::net_qa) map:          Option<ResMut<'w, EditorMap>>,
    pub(in crate::net_qa) terrain:      Option<ResMut<'w, TerrainDraft>>,
    pub(in crate::net_qa) theme:        Option<ResMut<'w, ThemeDraft>>,
    pub(in crate::net_qa) gang:         Option<ResMut<'w, GangDraft>>,
    pub(in crate::net_qa) armor:        Option<ResMut<'w, ArmorDraft>>,
    pub(in crate::net_qa) injury:       Option<ResMut<'w, InjuryDraft>>,
    pub(in crate::net_qa) sprite:       Option<ResMut<'w, SpriteDraft>>,
    pub(in crate::net_qa) attachment:   Option<ResMut<'w, AttachmentDraft>>,
    pub(in crate::net_qa) weapon:       Option<ResMut<'w, WeaponDraft>>,
    pub(in crate::net_qa) melee_weapon: Option<ResMut<'w, MeleeWeaponDraft>>,
}
