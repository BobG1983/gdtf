//! Cross-check the sprite key each authored terrain view names against the sprite registry.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;

use crate::sprites::{SpriteDefRegistry, SpriteName};

/// Record dangling `views[].sprite` → sprite references, one per authored row.
pub fn check_terrain_view_sprite_refs(
    terrain: Res<TerrainDefRegistry>,
    sprites: Res<SpriteDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        for row in def.views.iter() {
            if sprites.contains(&SpriteName::new((*row.sprite).clone())) {
                continue;
            }
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "terrain def `{}` ({}) view {:?}",
                    *def.display_name, **key, row.view,
                )),
                target:   FindingTarget::new((*row.sprite).clone()),
                family:   FindingFamily::new("SpriteDefRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
        }
    }
}
