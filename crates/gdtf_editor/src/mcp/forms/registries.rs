//! Every registry a QA load reads a key from, or a QA save resolves a name against.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::InjuryRegistry,
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

/// The loaded content registries the editor's forms read from.
#[derive(SystemParam)]
pub(in crate::mcp) struct EditorRegistries<'w> {
    pub(in crate::mcp) themes:       Option<Res<'w, UuidThemeRegistry>>,
    pub(in crate::mcp) terrain:      Option<Res<'w, TerrainDefRegistry>>,
    pub(in crate::mcp) gangs:        Option<Res<'w, GangRegistry>>,
    pub(in crate::mcp) armor:        Option<Res<'w, ArmorRegistry>>,
    pub(in crate::mcp) injuries:     Option<Res<'w, InjuryRegistry>>,
    pub(in crate::mcp) sprites:      Option<Res<'w, SpriteDefRegistry>>,
    pub(in crate::mcp) attachments:  Option<Res<'w, AttachmentRegistry>>,
    pub(in crate::mcp) weapons:      Option<Res<'w, WeaponRegistry>>,
    pub(in crate::mcp) melee_weapon: Option<Res<'w, MeleeWeaponRegistry>>,
    pub(in crate::mcp) fields:       Option<Res<'w, FieldDefRegistry>>,
}
