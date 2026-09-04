//! Every in-progress draft a QA write command may blank, load, or save.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    editor_map::EditorMap,
    field_form::FieldDraft,
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

/// The authoring session and the twelve drafts the editor's forms hold, all scoped to Editing.
#[derive(SystemParam)]
pub(in crate::mcp) struct EditorForms<'w> {
    pub(in crate::mcp) session:      Option<ResMut<'w, MapEditorSession>>,
    pub(in crate::mcp) map:          Option<ResMut<'w, EditorMap>>,
    pub(in crate::mcp) terrain:      Option<ResMut<'w, TerrainDraft>>,
    pub(in crate::mcp) theme:        Option<ResMut<'w, ThemeDraft>>,
    pub(in crate::mcp) gang:         Option<ResMut<'w, GangDraft>>,
    pub(in crate::mcp) armor:        Option<ResMut<'w, ArmorDraft>>,
    pub(in crate::mcp) injury:       Option<ResMut<'w, InjuryDraft>>,
    pub(in crate::mcp) weighting:    Option<ResMut<'w, WeightingDraft>>,
    pub(in crate::mcp) sprite:       Option<ResMut<'w, SpriteDraft>>,
    pub(in crate::mcp) attachment:   Option<ResMut<'w, AttachmentDraft>>,
    pub(in crate::mcp) weapon:       Option<ResMut<'w, WeaponDraft>>,
    pub(in crate::mcp) melee_weapon: Option<ResMut<'w, MeleeWeaponDraft>>,
    pub(in crate::mcp) field:        Option<ResMut<'w, FieldDraft>>,
}
