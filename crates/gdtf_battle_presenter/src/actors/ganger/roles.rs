use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use gdtf_battle_sim::prelude::Faction;
use serde::Deserialize;

use crate::TileIndex;

#[derive(Resource, Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct CharacterRoles {
        pub faction_0: TileIndex,
            pub faction_1: TileIndex,
}

impl CharacterRoles {
                                    #[must_use]
    pub fn base_for(&self, faction: Faction) -> TileIndex {
        match *faction {
            1 => self.faction_1,
            _ => self.faction_0,
        }
    }
}

const CHARACTER_ROLES_RON_PATH: &str = "sprites/character_roles.spritedef.ron";

pub(crate) fn register_character_roles_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<CharacterRoles>(CHARACTER_ROLES_RON_PATH);
}
