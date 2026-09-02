//! The shell system's per-mode **model-borrow bundles** — one `#[derive(SystemParam)]`
#[cfg(debug_assertions)]
use std::path::PathBuf;

use bevy::prelude::*;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::{PrefabRegistry, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

#[cfg(debug_assertions)]
use crate::net_qa::EditorQaAssetsRoot;
use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    canvas::{CanvasZoom, CurrentEditLevel},
    editor_map::EditorMap,
    egui_shell::{prefab::level_rail::RailUiState, sprite_form_ui::SpritePreviewCache},
    field_form::FieldDraft,
    gang_form::GangDraft,
    hovered_cell::HoveredCell,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    mode::{EditorMode, InjurySubTab},
    preview::{target::PreviewTarget, view::PreviewPan},
    save_record::LastSaveRecord,
    session::MapEditorSession,
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

/// The workbench-wide state every mode panel reads: the open tab, the session, the last save.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Workbench<'w> {
    pub(super) mode:      Option<ResMut<'w, EditorMode>>,
    pub(super) session:   Option<ResMut<'w, MapEditorSession>>,
    pub(super) last_save: ResMut<'w, LastSaveRecord>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SharedRegistries<'w> {
    pub(super) themes:  Option<Res<'w, UuidThemeRegistry>>,
    pub(super) terrain: Option<Res<'w, TerrainDefRegistry>>,
    pub(super) weapons: Option<Res<'w, WeaponRegistry>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PrefabParams<'w, 's> {
    pub(super) map:            Option<ResMut<'w, EditorMap>>,
    pub(super) edit_level:     Option<ResMut<'w, CurrentEditLevel>>,
    pub(super) hovered:        Option<ResMut<'w, HoveredCell>>,
    pub(super) zoom:           Option<ResMut<'w, CanvasZoom>>,
    pub(super) pan:            Option<ResMut<'w, PreviewPan>>,
    pub(super) view:           Option<ResMut<'w, ViewMode>>,
    pub(super) isolate:        Option<ResMut<'w, IsolateView>>,
    pub(super) preview_target: Option<Res<'w, PreviewTarget>>,
    pub(super) prefabs:        Option<Res<'w, PrefabRegistry>>,
    pub(super) rail_state:     Local<'s, RailUiState>,
    pub(super) save_name:      Local<'s, String>,
}

/// The in-progress terrain and theme edits the map-authoring modes hold.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TerrainThemeDrafts<'w> {
    pub(super) terrain: Option<ResMut<'w, TerrainDraft>>,
    pub(super) theme:   Option<ResMut<'w, ThemeDraft>>,
}

/// The draft and registries behind each content form the shell draws.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ContentForms<'w, 's> {
    pub(super) gang:         GangParams<'w>,
    pub(super) armor:        ArmorParams<'w>,
    pub(super) injury:       InjuryParams<'w>,
    pub(super) sprite:       SpriteParams<'w, 's>,
    pub(super) attachment:   AttachmentParams<'w>,
    pub(super) weapon:       WeaponParams<'w>,
    pub(super) melee_weapon: MeleeWeaponParams<'w>,
    pub(super) field:        FieldParams<'w>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GangParams<'w> {
    pub(super) draft:  Option<ResMut<'w, GangDraft>>,
    pub(super) gangs:  Option<Res<'w, GangRegistry>>,
    pub(super) melee:  Option<Res<'w, MeleeWeaponRegistry>>,
    pub(super) armor:  Option<Res<'w, ArmorRegistry>>,
    pub(super) tuning: Option<Res<'w, GangerStatTuning>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArmorParams<'w> {
    pub(super) draft:    Option<ResMut<'w, ArmorDraft>>,
    pub(super) registry: Option<Res<'w, ArmorRegistry>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct InjuryParams<'w> {
    pub(super) sub_tab:   Option<ResMut<'w, InjurySubTab>>,
    pub(super) draft:     Option<ResMut<'w, InjuryDraft>>,
    pub(super) weighting: Option<ResMut<'w, WeightingDraft>>,
    pub(super) registry:  Option<Res<'w, InjuryRegistry>>,
    pub(super) tables:    Option<Res<'w, InjuryTables>>,
    #[cfg(debug_assertions)]
    pub(super) qa_root:   Res<'w, EditorQaAssetsRoot>,
}

#[cfg(debug_assertions)]
impl InjuryParams<'_> {
    /// The one root every weighting save writes under, seeded by `MapEditorPlugin`.
    pub(super) fn assets_root(&self) -> PathBuf {
        self.qa_root.to_path_buf()
    }
}

/// tolerate their absence; the asset surfaces the PREVIEW needs (the image store + the
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SpriteParams<'w, 's> {
    pub(super) draft:         Option<ResMut<'w, SpriteDraft>>,
    pub(super) registry:      Option<Res<'w, SpriteDefRegistry>>,
    pub(super) images:        Option<Res<'w, Assets<Image>>>,
    pub(super) asset_server:  Option<Res<'w, AssetServer>>,
    pub(super) preview_cache: Local<'s, SpritePreviewCache>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct AttachmentParams<'w> {
    pub(super) draft:    Option<ResMut<'w, AttachmentDraft>>,
    pub(super) registry: Option<Res<'w, AttachmentRegistry>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct WeaponParams<'w> {
    pub(super) draft:       Option<ResMut<'w, WeaponDraft>>,
    pub(super) registry:    Option<Res<'w, WeaponRegistry>>,
    pub(super) attachments: Option<Res<'w, AttachmentRegistry>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MeleeWeaponParams<'w> {
    pub(super) draft:       Option<ResMut<'w, MeleeWeaponDraft>>,
    pub(super) registry:    Option<Res<'w, MeleeWeaponRegistry>>,
    pub(super) attachments: Option<Res<'w, AttachmentRegistry>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct FieldParams<'w> {
    pub(super) draft:    Option<ResMut<'w, FieldDraft>>,
    pub(super) registry: Option<Res<'w, FieldDefRegistry>>,
}
