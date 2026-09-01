//! Load an existing terrain def into the form draft.

use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainPresenterKind, TerrainSimKind};

use super::{
    draft::TerrainDraft,
    picks::{FootfallChoice, TerrainKindChoice},
};

impl TerrainDraft {
    /// Fill the draft from an existing terrain def, including on-death.
    pub fn load_from_def(&mut self, def: &TerrainDef) {
        self.set_display_name((*def.display_name).clone());
        self.set_kind(TerrainKindChoice::from(def.sim_kind.kind()));
        match &def.sim_kind {
            TerrainSimKind::Wall {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
            }
            | TerrainSimKind::Cover {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
            } => {
                self.set_cover_hp(*hp);
                self.set_armor_protection(*armor_protection);
                self.set_armor_hardness(*armor_hardness);
                self.set_height_band(*height_band);
            }
            TerrainSimKind::Slab {
                hp,
                armor_protection,
                armor_hardness,
            } => {
                self.set_slab_hp(*hp);
                self.set_armor_protection(*armor_protection);
                self.set_armor_hardness(*armor_hardness);
            }
            TerrainSimKind::Emplacement {
                hp,
                armor_protection,
                armor_hardness,
                height_band,
                mounted_weapon,
                entry_sides,
            } => {
                self.set_cover_hp(*hp);
                self.set_armor_protection(*armor_protection);
                self.set_armor_hardness(*armor_hardness);
                self.set_height_band(*height_band);
                self.set_mounted_weapon(Some(mounted_weapon.clone()));
                self.set_entry_sides(entry_sides.clone());
            }
        }
        let graphic_name = match &def.presenter_kind {
            TerrainPresenterKind::Wall { graphic_name }
            | TerrainPresenterKind::Cover { graphic_name }
            | TerrainPresenterKind::Emplacement { graphic_name } => graphic_name,
            TerrainPresenterKind::Slab {
                graphic_name,
                footfall,
            } => {
                self.set_footfall(FootfallChoice::from_sound(footfall.as_ref()));
                graphic_name
            }
        };
        if let Some(role) = TileRole::from_key(graphic_name) {
            self.set_graphic(role);
        }
        self.replace_tags(def.tags.clone());
        self.set_blocks_pathing(def.blocks_pathing.map(|flag| *flag));
        self.set_blocks_los(def.blocks_los);
        self.replace_on_death(def.on_death.clone());
        self.set_uuid(Some(def.key));
    }
}
