use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::EditorState;

pub(crate) fn transition_to_editing(gate: GateResources, mut next: ResMut<NextState<EditorState>>) {
    if gate.all_present() {
        next.set(EditorState::Editing);
    }
}

/// `#[derive(SystemParam)]` (the shell's `PrefabParams` pattern) so the gate system's
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GateResources<'w> {
    weapons:       Option<Res<'w, WeaponRegistry>>,
    armor:         Option<Res<'w, ArmorRegistry>>,
    terrain_defs:  Option<Res<'w, TerrainDefRegistry>>,
    theme_defs:    Option<Res<'w, UuidThemeRegistry>>,
    gangs:         Option<Res<'w, GangRegistry>>,
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    injuries:      Option<Res<'w, InjuryRegistry>>,
    injury_tables: Option<Res<'w, InjuryTables>>,
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    attachments:   Option<Res<'w, AttachmentRegistry>>,
}

impl GateResources<'_> {
    const fn all_present(&self) -> bool {
        self.weapons.is_some()
            && self.armor.is_some()
            && self.terrain_defs.is_some()
            && self.theme_defs.is_some()
            && self.gangs.is_some()
            && self.melee_weapons.is_some()
            && self.injuries.is_some()
            && self.injury_tables.is_some()
            && self.sprite_defs.is_some()
            && self.attachments.is_some()
    }
}
