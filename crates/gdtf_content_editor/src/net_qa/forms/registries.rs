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
pub(in crate::net_qa) struct EditorRegistries<'w> {
    pub(in crate::net_qa) themes:       Option<Res<'w, UuidThemeRegistry>>,
    pub(in crate::net_qa) terrain:      Option<Res<'w, TerrainDefRegistry>>,
    pub(in crate::net_qa) gangs:        Option<Res<'w, GangRegistry>>,
    pub(in crate::net_qa) armor:        Option<Res<'w, ArmorRegistry>>,
    pub(in crate::net_qa) injuries:     Option<Res<'w, InjuryRegistry>>,
    pub(in crate::net_qa) sprites:      Option<Res<'w, SpriteDefRegistry>>,
    pub(in crate::net_qa) attachments:  Option<Res<'w, AttachmentRegistry>>,
    pub(in crate::net_qa) weapons:      Option<Res<'w, WeaponRegistry>>,
    pub(in crate::net_qa) melee_weapon: Option<Res<'w, MeleeWeaponRegistry>>,
    pub(in crate::net_qa) fields:       Option<Res<'w, FieldDefRegistry>>,
}
