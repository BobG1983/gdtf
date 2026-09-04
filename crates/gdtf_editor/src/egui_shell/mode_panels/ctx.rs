use bevy::prelude::ResMut;
use gdtf_assets::ContentSourcePaths;
use gdtf_battle_sim::{
    level::UuidThemeRegistry, terrain::def::TerrainDefRegistry, weapon::WeaponRegistry,
};
use gdtf_content_families::TerrainDefsFamily;

use crate::{
    egui_shell::{
        params::{
            ArmorParams, AttachmentParams, FieldParams, GangParams, InjuryParams,
            MeleeWeaponParams, PrefabParams, SpriteParams, WeaponParams,
        },
        textures::ResolvedTextures,
    },
    save_record::LastSaveRecord,
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
};

pub(in crate::egui_shell) struct ModePanelsCtx<
    'a,
    'sess,
    'ter,
    'theme,
    'prefab,
    'ps,
    'gang,
    'armor,
    'injury,
    'sprite,
    'ss,
    'attach,
    'weapon,
    'melee,
    'field,
> {
    pub(in crate::egui_shell) session:          &'a mut ResMut<'sess, MapEditorSession>,
    pub(in crate::egui_shell) last_save:        &'a mut LastSaveRecord,
    pub(in crate::egui_shell) terrain_draft:    &'a mut ResMut<'ter, TerrainDraft>,
    pub(in crate::egui_shell) theme_draft:      &'a mut ResMut<'theme, ThemeDraft>,
    pub(in crate::egui_shell) themes:           Option<&'a UuidThemeRegistry>,
    pub(in crate::egui_shell) terrain_registry: Option<&'a TerrainDefRegistry>,
    pub(in crate::egui_shell) terrain_sources:  Option<&'a ContentSourcePaths<TerrainDefsFamily>>,
    pub(in crate::egui_shell) weapons:          Option<&'a WeaponRegistry>,
    pub(in crate::egui_shell) textures:         &'a ResolvedTextures,
    pub(in crate::egui_shell) prefab:           &'a mut PrefabParams<'prefab, 'ps>,
    pub(in crate::egui_shell) gang:             &'a mut GangParams<'gang>,
    pub(in crate::egui_shell) armor:            &'a mut ArmorParams<'armor>,
    pub(in crate::egui_shell) injury:           &'a mut InjuryParams<'injury>,
    pub(in crate::egui_shell) sprite:           &'a mut SpriteParams<'sprite, 'ss>,
    pub(in crate::egui_shell) attachment:       &'a mut AttachmentParams<'attach>,
    pub(in crate::egui_shell) weapon:           &'a mut WeaponParams<'weapon>,
    pub(in crate::egui_shell) melee_weapon:     &'a mut MeleeWeaponParams<'melee>,
    pub(in crate::egui_shell) field:            &'a mut FieldParams<'field>,
    /// The families a delete is offered for, and what the drawn button asked for.
    #[cfg(feature = "mcp")]
    pub(in crate::egui_shell) deletes:          Option<&'a crate::delete::DeleteRegistry>,
    #[cfg(feature = "mcp")]
    pub(in crate::egui_shell) delete_request:   &'a mut Option<crate::delete::DeleteRequest>,
}
