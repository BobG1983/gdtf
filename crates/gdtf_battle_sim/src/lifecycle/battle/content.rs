//! The loaded content catalogs a battle spawns from.

use bevy::{ecs::system::SystemParam, prelude::Res};

use crate::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    terrain::def::TerrainDefRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// Every content registry a battle setup resolves authored keys against.
#[derive(SystemParam)]
pub struct BattleContent<'w> {
    pub(super) gangs:         Option<Res<'w, GangRegistry>>,
    pub(super) weapons:       Option<Res<'w, WeaponRegistry>>,
    pub(super) melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    pub(super) armor:         Option<Res<'w, ArmorRegistry>>,
    pub(super) terrain:       Option<Res<'w, TerrainDefRegistry>>,
    pub(super) stat_tuning:   Option<Res<'w, GangerStatTuning>>,
    pub(super) combat_tuning: Option<Res<'w, CombatTuning>>,
    pub(super) field_defs:    Option<Res<'w, FieldDefRegistry>>,
    pub(super) attachments:   Option<Res<'w, AttachmentRegistry>>,
}
