//! Cross-check terrain graphic names against the sprite registry.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainPresenterKind};

use crate::sprites::{SpriteDefRegistry, SpriteName};

/// Record dangling terrain `graphic_name` → sprite references.
pub fn check_terrain_graphic_refs(
    terrain: Res<TerrainDefRegistry>,
    sprites: Res<SpriteDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        let graphic_name = match &def.presenter_kind {
            TerrainPresenterKind::Wall { graphic_name }
            | TerrainPresenterKind::Cover { graphic_name }
            | TerrainPresenterKind::Emplacement { graphic_name }
            | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
        };
        if !sprites.contains(&SpriteName::new((**graphic_name).clone())) {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "terrain def `{}` ({}) graphic_name",
                    *def.display_name, **key,
                )),
                target: FindingTarget::new((**graphic_name).clone()),
                family: FindingFamily::new("SpriteDefRegistry".to_owned()),
                scheme: ReferenceKeyScheme::FileStem,
            });
        }
    }
}
