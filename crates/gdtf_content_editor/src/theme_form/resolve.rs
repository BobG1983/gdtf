use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDefRegistry, TerrainSimKind, TerrainUuid},
    entity::TerrainPieceKind,
};

use super::types::ThemeDraft;

const HP_BAR_CEILING: f32 = 1000.0;

#[must_use]
pub fn resolved_stats(def: &TerrainDef) -> (String, f32) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "structural HP is a small u32 pool well within f32 exact range; the readout shows \
                  a rounded HP and an HP-bar fraction"
    )]
    let (kind, hp, protection, hardness, band) = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Wall",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Cover",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Slab {
            hp,
            armor_protection,
            armor_hardness,
        } => (
            "Slab",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            None,
        ),
        TerrainSimKind::Emplacement {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
            ..
        } => (
            "Emplacement",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
    };
    let name = (*def.display_name).clone();
    let band_line = band.map_or_else(String::new, |b| format!("\nBand: {b}"));
    let summary = format!(
        "{name}\n{kind}\nHP: {hp:.0}\nArmor: {protection}\nHardness: {hardness}{band_line}"
    );
    (summary, hp / HP_BAR_CEILING)
}

#[must_use]
pub fn floor_candidates(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
) -> Vec<(TerrainUuid, String)> {
    let mut slabs: Vec<(TerrainUuid, String)> = Vec::new();
    let mut others: Vec<(TerrainUuid, String)> = Vec::new();
    for key in draft.terrain() {
        let Some(def) = terrain.def(key) else {
            continue;
        };
        let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
        if def.sim_kind.kind() == TerrainPieceKind::Slab {
            slabs.push((*key, label));
        } else {
            others.push((*key, label));
        }
    }
    slabs.extend(others);
    slabs
}

#[must_use]
pub fn slab_floor_candidates(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
) -> Vec<(TerrainUuid, String)> {
    draft
        .terrain()
        .iter()
        .filter_map(|key| {
            let def = terrain.def(key)?;
            if def.sim_kind.kind() == TerrainPieceKind::Slab {
                let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
                Some((*key, label))
            } else {
                None
            }
        })
        .collect()
}

#[must_use]
pub(crate) const fn sim_kind_label(kind: &TerrainSimKind) -> &'static str {
    match kind.kind() {
        TerrainPieceKind::Wall => "Wall",
        TerrainPieceKind::Cover => "Cover",
        TerrainPieceKind::Slab => "Slab",
        TerrainPieceKind::Emplacement => "Emplacement",
    }
}
