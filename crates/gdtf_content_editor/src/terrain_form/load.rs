//! Load an existing terrain def into the form draft, and the rows a picker offers.

use std::collections::HashMap;

use gdtf_battle_sim::{
    cover::CoverHp,
    slab::SlabHp,
    terrain::def::{
        TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainSimKind, TerrainUuid,
    },
};

use super::{
    draft::TerrainDraft,
    picks::{FootfallChoice, TerrainKindChoice},
};

/// Every terrain def as the (key, label) row the load picker draws, sorted by display name.
/// A display name two or more defs share carries that def's key, so no two rows read alike.
#[must_use]
pub fn load_candidates(registry: &TerrainDefRegistry) -> Vec<(TerrainUuid, String)> {
    let mut held: HashMap<String, usize> = HashMap::new();
    for (_, def) in registry.defs() {
        *held.entry((*def.display_name).clone()).or_insert(0) += 1;
    }
    let mut rows: Vec<(TerrainUuid, String, String)> = registry
        .defs()
        .map(|(key, def)| (*key, (*def.display_name).clone(), (**key).to_string()))
        .collect();
    rows.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(&right.2)));
    rows.into_iter()
        .map(|(key, name, text)| {
            let label = if held.get(&name).is_some_and(|count| *count > 1) {
                format!("{name}  [{text}]")
            } else {
                name
            };
            (key, label)
        })
        .collect()
}

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
                self.set_slab_hp(SlabHp::new(**hp));
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
                self.set_cover_hp(CoverHp::new(**hp));
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
                self.set_slab_hp(SlabHp::new(**hp));
                self.set_armor_protection(*armor_protection);
                self.set_armor_hardness(*armor_hardness);
                self.set_height_band(*height_band);
                self.set_mounted_weapon(Some(mounted_weapon.clone()));
                self.set_entry_sides(entry_sides.clone());
            }
        }
        match &def.presenter_kind {
            TerrainPresenterKind::Wall
            | TerrainPresenterKind::Cover
            | TerrainPresenterKind::Emplacement => {}
            TerrainPresenterKind::Slab { footfall, .. } => {
                self.set_footfall(FootfallChoice::from_sound(footfall.as_ref()));
            }
        }
        self.replace_views(def.views.clone());
        self.replace_tags(def.tags.clone());
        self.set_blocks_pathing(def.blocks_pathing.map(|flag| *flag));
        self.set_blocks_los(def.blocks_los);
        self.set_leaves_behind(def.leaves_behind.clone());
        self.replace_on_death(def.on_death.clone());
        self.set_uuid(Some(def.key));
    }
}
